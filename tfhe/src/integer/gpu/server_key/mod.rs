use crate::core_crypto::gpu::lwe_bootstrap_key::CudaLweBootstrapKey;
use crate::core_crypto::gpu::lwe_keyswitch_key::CudaLweKeyswitchKey;
use crate::core_crypto::gpu::lwe_multi_bit_bootstrap_key::CudaLweMultiBitBootstrapKey;
use crate::core_crypto::gpu::{CudaDevice, CudaStream};
use crate::core_crypto::prelude::{
    allocate_and_generate_new_lwe_keyswitch_key, par_allocate_and_generate_new_lwe_bootstrap_key,
    par_allocate_and_generate_new_lwe_multi_bit_bootstrap_key, LweBootstrapKeyOwned,
    LweMultiBitBootstrapKeyOwned,
};
use crate::integer::ClientKey;
use crate::shortint::ciphertext::{MaxDegree, MaxNoiseLevel};
use crate::shortint::engine::ShortintEngine;
use crate::shortint::{CarryModulus, CiphertextModulus, MessageModulus, PBSOrder};

mod radix;

pub enum CudaBootstrappingKey {
    Classic(CudaLweBootstrapKey),
    MultiBit(CudaLweMultiBitBootstrapKey),
}

/// A structure containing the server public key.
///
/// The server key is generated by the client and is meant to be published: the client
/// sends it to the server so it can compute homomorphic circuits.
// #[derive(PartialEq, Serialize, Deserialize)]
pub struct CudaServerKey {
    pub key_switching_key: CudaLweKeyswitchKey<u64>,
    pub bootstrapping_key: CudaBootstrappingKey,
    // Size of the message buffer
    pub message_modulus: MessageModulus,
    // Size of the carry buffer
    pub carry_modulus: CarryModulus,
    // Maximum number of operations that can be done before emptying the operation buffer
    pub max_degree: MaxDegree,
    pub max_noise_level: MaxNoiseLevel,
    // Modulus use for computations on the ciphertext
    pub ciphertext_modulus: CiphertextModulus,
    pub pbs_order: PBSOrder,
}

impl CudaServerKey {
    /// Generates a server key that stores keys in the device memory.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tfhe::core_crypto::gpu::{CudaDevice, CudaStream};
    /// use tfhe::integer::gpu::CudaServerKey;
    /// use tfhe::integer::ClientKey;
    /// use tfhe::shortint::parameters::PARAM_MESSAGE_2_CARRY_2_KS_PBS;
    ///
    /// let gpu_index = 0;
    /// let device = CudaDevice::new(gpu_index);
    /// let mut stream = CudaStream::new_unchecked(device);
    ///
    /// // Generate the client key:
    /// let cks = ClientKey::new(PARAM_MESSAGE_2_CARRY_2_KS_PBS);
    ///
    /// // Generate the server key:
    /// let sks = CudaServerKey::new(&cks, &mut stream);
    /// ```
    pub fn new<C>(cks: C, stream: &CudaStream) -> Self
    where
        C: AsRef<ClientKey>,
    {
        // It should remain just enough space to add a carry
        let client_key = cks.as_ref();
        let max_degree = MaxDegree::integer_radix_server_key(
            client_key.key.parameters.message_modulus(),
            client_key.key.parameters.carry_modulus(),
        );
        Self::new_server_key_with_max_degree(client_key, max_degree, stream)
    }

    pub(crate) fn new_server_key_with_max_degree(
        cks: &ClientKey,
        max_degree: MaxDegree,
        stream: &CudaStream,
    ) -> Self {
        let mut engine = ShortintEngine::new();

        // Generate a regular keyset and convert to the GPU
        let pbs_params_base = &cks.parameters();
        let d_bootstrapping_key = match pbs_params_base {
            crate::shortint::PBSParameters::PBS(pbs_params) => {
                let h_bootstrap_key: LweBootstrapKeyOwned<u64> =
                    par_allocate_and_generate_new_lwe_bootstrap_key(
                        &cks.key.small_lwe_secret_key(),
                        &cks.key.glwe_secret_key,
                        pbs_params.pbs_base_log,
                        pbs_params.pbs_level,
                        pbs_params.glwe_noise_distribution,
                        pbs_params.ciphertext_modulus,
                        &mut engine.encryption_generator,
                    );

                let d_bootstrap_key =
                    CudaLweBootstrapKey::from_lwe_bootstrap_key(&h_bootstrap_key, stream);

                CudaBootstrappingKey::Classic(d_bootstrap_key)
            }
            crate::shortint::PBSParameters::MultiBitPBS(pbs_params) => {
                let h_bootstrap_key: LweMultiBitBootstrapKeyOwned<u64> =
                    par_allocate_and_generate_new_lwe_multi_bit_bootstrap_key(
                        &cks.key.small_lwe_secret_key(),
                        &cks.key.glwe_secret_key,
                        pbs_params.pbs_base_log,
                        pbs_params.pbs_level,
                        pbs_params.grouping_factor,
                        pbs_params.glwe_noise_distribution,
                        pbs_params.ciphertext_modulus,
                        &mut engine.encryption_generator,
                    );

                let d_bootstrap_key = CudaLweMultiBitBootstrapKey::from_lwe_multi_bit_bootstrap_key(
                    &h_bootstrap_key,
                    stream,
                );

                CudaBootstrappingKey::MultiBit(d_bootstrap_key)
            }
        };

        // Creation of the key switching key
        let h_key_switching_key = allocate_and_generate_new_lwe_keyswitch_key(
            &cks.key.large_lwe_secret_key(),
            &cks.key.small_lwe_secret_key(),
            cks.parameters().ks_base_log(),
            cks.parameters().ks_level(),
            cks.parameters().lwe_noise_distribution(),
            cks.parameters().ciphertext_modulus(),
            &mut engine.encryption_generator,
        );

        let d_key_switching_key =
            CudaLweKeyswitchKey::from_lwe_keyswitch_key(&h_key_switching_key, stream);

        assert!(matches!(
            cks.parameters().encryption_key_choice().into(),
            PBSOrder::KeyswitchBootstrap
        ));

        // Pack the keys in the server key set:
        Self {
            key_switching_key: d_key_switching_key,
            bootstrapping_key: d_bootstrapping_key,
            message_modulus: cks.parameters().message_modulus(),
            carry_modulus: cks.parameters().carry_modulus(),
            max_degree,
            max_noise_level: cks.parameters().max_noise_level(),
            ciphertext_modulus: cks.parameters().ciphertext_modulus(),
            pbs_order: cks.parameters().encryption_key_choice().into(),
        }
    }

    /// Decompress a CompressedServerKey to a CudaServerKey
    ///
    /// This is useful in particular for debugging purposes, as it allows to compare the result of
    /// CPU & GPU computations. When using trivial encryption it is then possible to track
    /// intermediate and final result values easily between CPU and GPU.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tfhe::core_crypto::gpu::{CudaDevice, CudaStream};
    /// use tfhe::integer::gpu::ciphertext::CudaUnsignedRadixCiphertext;
    /// use tfhe::integer::gpu::CudaServerKey;
    /// use tfhe::integer::{ClientKey, CompressedServerKey, ServerKey};
    /// use tfhe::shortint::parameters::PARAM_MESSAGE_2_CARRY_2_KS_PBS;
    ///
    /// let gpu_index = 0;
    /// let device = CudaDevice::new(gpu_index);
    /// let mut stream = CudaStream::new_unchecked(device);
    /// let size = 4;
    /// let cks = ClientKey::new(PARAM_MESSAGE_2_CARRY_2_KS_PBS);
    /// let compressed_sks = CompressedServerKey::new_radix_compressed_server_key(&cks);
    /// let cuda_sks = CudaServerKey::decompress_from_cpu(&compressed_sks);
    /// let cpu_sks = compressed_sks.decompress();
    /// let msg = 1;
    /// let scalar = 3;
    /// let ct = cpu_sks.create_trivial_radix(msg, size);
    /// let d_ct = CudaUnsignedRadixCiphertext::from_radix_ciphertext(&ct, &mut stream);
    /// // Compute homomorphically a scalar multiplication:
    /// let d_ct_res = cuda_sks.unchecked_scalar_add(&d_ct, scalar, &mut stream);
    /// let ct_res = d_ct_res.to_radix_ciphertext(&mut stream);
    /// let ct_res_cpu = cpu_sks.unchecked_scalar_add(&ct, scalar);
    /// let clear: u64 = cks.decrypt_radix(&ct_res);
    /// let clear_cpu: u64 = cks.decrypt_radix(&ct_res_cpu);
    /// assert_eq!((scalar + msg) % (4_u64.pow(size as u32)), clear_cpu);
    /// assert_eq!((scalar + msg) % (4_u64.pow(size as u32)), clear);
    /// ```
    pub fn decompress_from_cpu(cpu_key: &crate::integer::CompressedServerKey) -> Self {
        let crate::shortint::CompressedServerKey {
            key_switching_key,
            bootstrapping_key,
            message_modulus,
            carry_modulus,
            max_degree,
            max_noise_level,
            ciphertext_modulus,
            pbs_order,
        } = cpu_key.key.clone();

        let device = CudaDevice::new(0);
        let stream = CudaStream::new_unchecked(device);

        let h_key_switching_key = key_switching_key.par_decompress_into_lwe_keyswitch_key();
        let key_switching_key =
            crate::core_crypto::gpu::lwe_keyswitch_key::CudaLweKeyswitchKey::from_lwe_keyswitch_key(
                &h_key_switching_key,
                &stream,
            );
        let bootstrapping_key = match bootstrapping_key {
            crate::shortint::server_key::compressed::ShortintCompressedBootstrappingKey::Classic(h_bootstrap_key) => {
                let standard_bootstrapping_key =
                    h_bootstrap_key.par_decompress_into_lwe_bootstrap_key();

                let d_bootstrap_key =
                    crate::core_crypto::gpu::lwe_bootstrap_key::CudaLweBootstrapKey::from_lwe_bootstrap_key(&standard_bootstrapping_key, &stream);

                crate::integer::gpu::server_key::CudaBootstrappingKey::Classic(d_bootstrap_key)
            }
            crate::shortint::server_key::compressed::ShortintCompressedBootstrappingKey::MultiBit {
                seeded_bsk: bootstrapping_key,
                deterministic_execution: _,
            } => {
                let standard_bootstrapping_key =
                    bootstrapping_key.par_decompress_into_lwe_multi_bit_bootstrap_key();

                let d_bootstrap_key =
                    crate::core_crypto::gpu::lwe_multi_bit_bootstrap_key::CudaLweMultiBitBootstrapKey::from_lwe_multi_bit_bootstrap_key(
                        &standard_bootstrapping_key, &stream);

                crate::integer::gpu::server_key::CudaBootstrappingKey::MultiBit(d_bootstrap_key)
            }
        };

        Self {
            key_switching_key,
            bootstrapping_key,
            message_modulus,
            carry_modulus,
            max_degree,
            max_noise_level,
            ciphertext_modulus,
            pbs_order,
        }
    }
}
