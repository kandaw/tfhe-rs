use super::utils::*;
use crate::c_api::high_level_api::booleans::FheBool;
use crate::c_api::high_level_api::i128::I128;
use crate::c_api::high_level_api::i256::I256;
use crate::c_api::high_level_api::u1024::U1024;
use crate::c_api::high_level_api::u128::U128;
use crate::c_api::high_level_api::u2048::U2048;
use crate::c_api::high_level_api::u256::U256;
use crate::c_api::high_level_api::u512::U512;
use crate::high_level_api::prelude::*;
use std::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div, DivAssign,
    Mul, MulAssign, Neg, Not, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
};
use std::os::raw::c_int;

macro_rules! define_all_cast_into_for_integer_type {
    ($from:ty) => {
        define_casting_operation!($from =>
            FheUint2, FheUint4, FheUint6, FheUint8, FheUint10, FheUint14, FheUint16, FheUint32, FheUint64, FheUint128, FheUint160, FheUint256,
            FheInt2, FheInt4, FheInt6, FheInt8, FheInt10, FheInt12, FheInt14, FheInt16, FheInt32, FheInt64, FheInt128, FheInt160, FheInt256
        );
    };
}

/// Implement C functions for all the operations supported by a integer type,
/// which should also be accessible from C API
///
/// We require the shift amount to be an unsigned type,
/// so to be able to use that macro for signed integers (which have a signed clear type)
/// we also accept an additional clear_shift_type
macro_rules! impl_operations_for_integer_type {
    (
        name: $name:ident,
        fhe_unsigned_type: $fhe_unsigned_type:ty,
        clear_scalar_type: $clear_scalar_type:ty,
        clear_shift_type: $clear_shift_type:ty
        $(,)?
    ) => {
        impl_binary_fn_on_type!($name =>
            add,
            sub,
            mul,
            bitand,
            bitor,
            bitxor,
            min,
            max,
            div,
            rem,
        );

        impl_binary_overflowing_fn_on_type!($name =>
            overflowing_add,
            overflowing_sub,
            overflowing_mul,
        );

        // Handle comparisons separately as they return FheBool
        impl_comparison_fn_on_type!(
            lhs_type: $name,
            rhs_type: $name,
            comparison_fn_names: eq, ne, ge, gt, le, lt,
        );

        // Handle comparisons separately as they return FheBool
        impl_scalar_comparison_fn_on_type!(
            lhs_type: $name,
            clear_type: $clear_scalar_type,
            comparison_fn_names: eq, ne, ge, gt, le, lt,
        );

        // handle shift separately as they require
        // rhs to be an unsigned type
        impl_binary_fn_on_type!(
            lhs_type: $name,
            rhs_type: $fhe_unsigned_type,
            binary_fn_names: shl, shr, rotate_right, rotate_left,
        );

        impl_binary_assign_fn_on_type!($name =>
            add_assign,
            sub_assign,
            mul_assign,
            bitand_assign,
            bitor_assign,
            bitxor_assign,
            div_assign,
            rem_assign,
        );

        // handle shift separately as they require
        // rhs to be an unsigned type
        impl_binary_assign_fn_on_type!(
            lhs_type: $name,
            rhs_type: $fhe_unsigned_type,
            binary_fn_names: shl_assign, shr_assign, rotate_right_assign, rotate_left_assign,
        );

        impl_scalar_binary_fn_on_type!($name, $clear_scalar_type =>
            add,
            sub,
            mul,
            bitand,
            bitor,
            bitxor,
            min,
            max,
            div,
            rem,
        );
        impl_scalar_binary_assign_fn_on_type!($name, $clear_scalar_type =>
            add_assign,
            sub_assign,
            mul_assign,
            bitand_assign,
            bitor_assign,
            bitxor_assign,
            div_assign,
            rem_assign,
        );

        // handle shift separately as they require
        // rhs to be an unsigned type
        impl_scalar_binary_fn_on_type!($name, $clear_shift_type =>
            shl,
            shr,
            rotate_right,
            rotate_left,
        );
        impl_scalar_binary_assign_fn_on_type!($name, $clear_shift_type =>
            shl_assign,
            shr_assign,
            rotate_right_assign,
            rotate_left_assign,
        );

        impl_unary_fn_on_type!($name => neg, not);
        impl_unary_fn_on_type!(
            input_type: $name,
            output_type: $crate::c_api::high_level_api::integers::FheUint32,
            unary_fn_names:
                /// Returns the number of leading zeros in the binary representation of input.
                leading_zeros,
                /// Returns the number of leading ones in the binary representation of input.
                leading_ones,
                /// Returns the number of trailing zeros in the binary representation of input.
                trailing_zeros,
                /// Returns the number of trailing ones in the binary representation of input.
                trailing_ones,
                /// Returns the base 2 logarithm of the number, rounded down.
                ///
                /// Result has no meaning if self encrypts a value that is <= 0.
                /// See `checked_ilog2`
                ilog2
        );
        impl_unary_fn_with_2_outputs_on_type!(
            input_type: $name,
            output_type_1: $crate::c_api::high_level_api::integers::FheUint32,
            output_type_2: $crate::c_api::high_level_api::booleans::FheBool,
            unary_fn_names:
                /// Returns the base 2 logarithm of the number, rounded down.
                ///
                /// Also returns a boolean flag that is true if the result is valid (i.e input was > 0)
                checked_ilog2
        );

        // Implement sum of many ciphertexts
        ::paste::paste! {
            #[no_mangle]
            pub unsafe extern "C" fn [<$name:snake _sum>](
                lhs: *const *const $name,
                len: usize,
                out_result: *mut *mut $name,
            ) -> c_int {
                $crate::c_api::utils::catch_panic(|| {
                    let slice_of_c_ptrs: &'_ [*const $name] = ::std::slice::from_raw_parts(lhs, len);
                    let result = slice_of_c_ptrs
                        .into_iter()
                        .map(|ptr| &(*(*ptr)).0)
                        .sum();
                    *out_result = Box::into_raw(Box::new($name(result)));
                })
            }
        }

        // Implement div_rem.
        // We can't use the macro above as div_rem returns a tuple.
        //
        // (Having div_rem is important for the cases where you need both
        // the quotient and remainder as you may save time by using the div_rem
        // instead of div and rem separately
        ::paste::paste! {
            #[no_mangle]
            pub unsafe extern "C" fn [<$name:snake _scalar_div_rem>](
                lhs: *const $name,
                rhs: $clear_scalar_type,
                q_result: *mut *mut $name,
                r_result: *mut *mut $name,
            ) -> c_int {
                $crate::c_api::utils::catch_panic(|| {
                    let lhs = $crate::c_api::utils::get_ref_checked(lhs).unwrap();
                    let rhs = <$clear_scalar_type as $crate::c_api::high_level_api::utils::CApiIntegerType
                        >::to_rust(rhs);

                    let (q, r) = (&lhs.0).div_rem(rhs);

                    *q_result = Box::into_raw(Box::new($name(q)));
                    *r_result = Box::into_raw(Box::new($name(r)));
                })
            }
        }

        ::paste::paste! {
            #[no_mangle]
            pub unsafe extern "C" fn [<$name:snake _div_rem>](
                lhs: *const $name,
                rhs: *const $name,
                q_result: *mut *mut $name,
                r_result: *mut *mut $name,
            ) -> c_int {
                $crate::c_api::utils::catch_panic(|| {
                    let lhs = $crate::c_api::utils::get_ref_checked(lhs).unwrap();
                    let rhs = $crate::c_api::utils::get_ref_checked(rhs).unwrap();

                    let (q, r) = (&lhs.0).div_rem(&rhs.0);

                    *q_result = Box::into_raw(Box::new($name(q)));
                    *r_result = Box::into_raw(Box::new($name(r)));
                })
            }
        }

        // Even though if_then_else/cmux is a method of FheBool, it still takes as
        // integers inputs, so its easier to keep the definition here
        ::paste::paste! {
            #[no_mangle]
            pub unsafe extern "C" fn [<$name:snake _if_then_else>](
                condition_ct: *const FheBool,
                then_ct: *const $name,
                else_ct: *const $name,
                result: *mut *mut $name,
            ) -> c_int {
                $crate::c_api::utils::catch_panic(|| {
                    let condition_ct = &$crate::c_api::utils::get_ref_checked(condition_ct).unwrap().0;
                    let then_ct = &$crate::c_api::utils::get_ref_checked(then_ct).unwrap().0;
                    let else_ct = &$crate::c_api::utils::get_ref_checked(else_ct).unwrap().0;

                    let r = condition_ct.if_then_else(then_ct, else_ct);

                    *result = Box::into_raw(Box::new($name(r)));
                })
            }

            // map cmux to if_then_else
            pub unsafe extern "C" fn [<$name:snake _cmux>](
                condition_ct: *const FheBool,
                then_ct: *const $name,
                else_ct: *const $name,
                result: *mut *mut $name,
            ) -> c_int {
                [<$name:snake _if_then_else>](condition_ct, then_ct, else_ct, result)
            }
        }
    };
}

/// Creates a type that will act as an opaque wrapper
/// around a tfhe integer.
///
/// It also implements binary operations for this wrapper type
macro_rules! create_integer_wrapper_type {

    (
        name: $name:ident,
        fhe_unsigned_type: $fhe_unsigned_type:ty,
        clear_scalar_type: $clear_scalar_type:ty,
        clear_shift_type: $clear_shift_type:ty
        $(,)?
    ) => {

        pub struct $name(pub(in $crate::c_api) $crate::high_level_api::$name);

        impl_destroy_on_type!($name);

        impl_operations_for_integer_type!(
            name: $name,
            fhe_unsigned_type: $fhe_unsigned_type,
            clear_scalar_type: $clear_scalar_type,
            clear_shift_type: $clear_shift_type,
        );

        impl_try_encrypt_trivial_on_type!($name{crate::high_level_api::$name}, $clear_scalar_type);

        impl_try_encrypt_with_client_key_on_type!($name{crate::high_level_api::$name}, $clear_scalar_type);

        impl_try_encrypt_with_public_key_on_type!($name{crate::high_level_api::$name}, $clear_scalar_type);

        impl_decrypt_on_type!($name, $clear_scalar_type);

        impl_try_decrypt_trivial_on_type!($name, $clear_scalar_type);

        impl_serialize_deserialize_on_type!($name);

        impl_clone_on_type!($name);

        impl_safe_serialize_on_type!($name);

        ::paste::paste! {
            impl_safe_deserialize_conformant_integer!($name, [<$name ConformanceParams>]);
        }

        define_all_cast_into_for_integer_type!($name);

        // The compressed version of the ciphertext type
        ::paste::paste! {
            pub struct [<Compressed $name>]($crate::high_level_api::[<Compressed $name>]);

            impl_destroy_on_type!([<Compressed $name>]);

            impl_clone_on_type!([<Compressed $name>]);

            impl_try_encrypt_with_client_key_on_type!([<Compressed $name>]{crate::high_level_api::[<Compressed $name>]}, $clear_scalar_type);

            impl_serialize_deserialize_on_type!([<Compressed $name>]);

            impl_safe_serialize_on_type!([<Compressed $name>]);

            impl_safe_deserialize_conformant_integer!([<Compressed $name>],  [<$name ConformanceParams>]);


            #[no_mangle]
            pub unsafe extern "C" fn [<compressed_ $name:snake _decompress>](
                sself: *const [<Compressed $name>],
                result: *mut *mut $name,
            ) -> ::std::os::raw::c_int {
                $crate::c_api::utils::catch_panic(|| {
                    let compressed = $crate::c_api::utils::get_ref_checked(sself).unwrap();

                    let decompressed_inner = compressed.0.decompress();
                    *result = Box::into_raw(Box::new($name(decompressed_inner)));
                })
            }

            #[no_mangle]
            pub unsafe extern "C" fn [<$name:snake _compress>](
                sself: *const $name,
                result: *mut *mut [<Compressed $name>],
            ) -> ::std::os::raw::c_int {
                $crate::c_api::utils::catch_panic(|| {
                    let ct = $crate::c_api::utils::get_ref_checked(sself).unwrap();

                    let compressed_inner = ct.0.compress();

                    *result = Box::into_raw(Box::new([<Compressed $name>](compressed_inner)));
                })
            }
        }
    };

    // This entry point is meant for unsigned types
    (
        name: $name:ident,
        clear_scalar_type: $clear_scalar_type:ty
    ) => {
        create_integer_wrapper_type!(
            name: $name,
            fhe_unsigned_type: $name,
            clear_scalar_type: $clear_scalar_type,
            clear_shift_type: $clear_scalar_type,
        );
    };

}
create_integer_wrapper_type!(name: FheUint2, clear_scalar_type: u8);
create_integer_wrapper_type!(name: FheUint4, clear_scalar_type: u8);
create_integer_wrapper_type!(name: FheUint6, clear_scalar_type: u8);
create_integer_wrapper_type!(name: FheUint8, clear_scalar_type: u8);
create_integer_wrapper_type!(name: FheUint10, clear_scalar_type: u16);
create_integer_wrapper_type!(name: FheUint12, clear_scalar_type: u16);
create_integer_wrapper_type!(name: FheUint14, clear_scalar_type: u16);
create_integer_wrapper_type!(name: FheUint16, clear_scalar_type: u16);
create_integer_wrapper_type!(name: FheUint32, clear_scalar_type: u32);
create_integer_wrapper_type!(name: FheUint64, clear_scalar_type: u64);
create_integer_wrapper_type!(name: FheUint128, clear_scalar_type: U128);
create_integer_wrapper_type!(name: FheUint160, clear_scalar_type: U256);
create_integer_wrapper_type!(name: FheUint256, clear_scalar_type: U256);
create_integer_wrapper_type!(name: FheUint512, clear_scalar_type: U512);
create_integer_wrapper_type!(name: FheUint1024, clear_scalar_type: U1024);
create_integer_wrapper_type!(name: FheUint2048, clear_scalar_type: U2048);

create_integer_wrapper_type!(
    name: FheInt2,
    fhe_unsigned_type: FheUint2,
    clear_scalar_type: i8,
    clear_shift_type: u8,
);
create_integer_wrapper_type!(
    name: FheInt4,
    fhe_unsigned_type: FheUint4,
    clear_scalar_type: i8,
    clear_shift_type: u8,
);
create_integer_wrapper_type!(
    name: FheInt6,
    fhe_unsigned_type: FheUint6,
    clear_scalar_type: i8,
    clear_shift_type: u8,
);
create_integer_wrapper_type!(
    name: FheInt8,
    fhe_unsigned_type: FheUint8,
    clear_scalar_type: i8,
    clear_shift_type: u8,
);
create_integer_wrapper_type!(
    name: FheInt10,
    fhe_unsigned_type: FheUint10,
    clear_scalar_type: i16,
    clear_shift_type: u16,
);
create_integer_wrapper_type!(
    name: FheInt12,
    fhe_unsigned_type: FheUint12,
    clear_scalar_type: i16,
    clear_shift_type: u16,
);
create_integer_wrapper_type!(
    name: FheInt14,
    fhe_unsigned_type: FheUint14,
    clear_scalar_type: i16,
    clear_shift_type: u16,
);
create_integer_wrapper_type!(
    name: FheInt16,
    fhe_unsigned_type: FheUint16,
    clear_scalar_type: i16,
    clear_shift_type: u16,
);
create_integer_wrapper_type!(
    name: FheInt32,
    fhe_unsigned_type: FheUint32,
    clear_scalar_type: i32,
    clear_shift_type: u32,
);
create_integer_wrapper_type!(
    name: FheInt64,
    fhe_unsigned_type: FheUint64,
    clear_scalar_type: i64,
    clear_shift_type: u64,
);
create_integer_wrapper_type!(
    name: FheInt128,
    fhe_unsigned_type: FheUint128,
    clear_scalar_type: I128,
    clear_shift_type: U128,
);
create_integer_wrapper_type!(
    name: FheInt160,
    fhe_unsigned_type: FheUint160,
    clear_scalar_type: I256,
    clear_shift_type: U256,
);
create_integer_wrapper_type!(
    name: FheInt256,
    fhe_unsigned_type: FheUint256,
    clear_scalar_type: I256,
    clear_shift_type: U256,
);

define_all_cast_into_for_integer_type!(FheBool);

macro_rules! impl_oprf_for_uint {
    (
        name: $name:ident
    ) => {

        ::paste::paste! {
            #[no_mangle]
            pub unsafe extern "C" fn [<generate_oblivious_pseudo_random_ $name:snake>](
                out_result: *mut *mut $name,
                seed_low_bytes: u64,
                seed_high_bytes: u64,
            ) -> c_int {
                use crate::high_level_api::IntegerId;
                $crate::c_api::utils::catch_panic(|| {
                    let seed_low_bytes: u128 = seed_low_bytes.into();
                    let seed_high_bytes: u128 = seed_high_bytes.into();
                    let seed = crate::Seed((seed_high_bytes << 64) | seed_low_bytes);

                    let result = crate::FheUint::generate_oblivious_pseudo_random(
                        seed,
                        <crate::[<$name Id>] as IntegerId>::num_bits() as u64
                    );
                    *out_result = Box::into_raw(Box::new($name(result)));
                })
            }
        }

        ::paste::paste! {
            #[no_mangle]
            pub unsafe extern "C" fn [<generate_oblivious_pseudo_random_bits_ $name:snake>](
                out_result: *mut *mut $name,
                seed_low_bytes: u64,
                seed_high_bytes: u64,
                random_bits_count: u64,
            ) -> c_int {

                $crate::c_api::utils::catch_panic(|| {
                    let seed_low_bytes: u128 = seed_low_bytes.into();
                    let seed_high_bytes: u128 = seed_high_bytes.into();
                    let seed = crate::Seed((seed_high_bytes << 64) | seed_low_bytes);

                    let result = crate::FheUint::generate_oblivious_pseudo_random(seed, random_bits_count);
                    *out_result = Box::into_raw(Box::new($name(result)));
                })
            }
        }
    };
}

macro_rules! impl_oprf_for_int {
    (
        name: $name:ident
    ) => {

        ::paste::paste! {
            #[no_mangle]
            pub unsafe extern "C" fn [<generate_oblivious_pseudo_random_unsigned_ $name:snake>](
                out_result: *mut *mut $name,
                seed_low_bytes: u64,
                seed_high_bytes: u64,
                random_bits_count: u64,
            ) -> c_int {
                $crate::c_api::utils::catch_panic(|| {
                    let seed_low_bytes: u128 = seed_low_bytes.into();
                    let seed_high_bytes: u128 = seed_high_bytes.into();
                    let seed = crate::Seed((seed_high_bytes << 64) | seed_low_bytes);

                    let result =
                        crate::FheInt::generate_oblivious_pseudo_random(
                            seed,
                            crate::high_level_api::SignedRandomizationSpec::Unsigned {
                                random_bits_count
                            },
                        );
                    *out_result = Box::into_raw(Box::new($name(result)));
                })
            }
        }

        ::paste::paste! {
            #[no_mangle]
            pub unsafe extern "C" fn [<generate_oblivious_pseudo_random_full_signed_range_ $name:snake>](
                out_result: *mut *mut $name,
                seed_low_bytes: u64,
                seed_high_bytes: u64,
            ) -> c_int {
                $crate::c_api::utils::catch_panic(|| {
                    let seed_low_bytes: u128 = seed_low_bytes.into();
                    let seed_high_bytes: u128 = seed_high_bytes.into();
                    let seed = crate::Seed((seed_high_bytes << 64) | seed_low_bytes);

                    let result = crate::FheInt::generate_oblivious_pseudo_random(
                        seed,
                        crate::high_level_api::SignedRandomizationSpec::FullSigned,
                    );
                    *out_result = Box::into_raw(Box::new($name(result)));
                })
            }
        }
    };
}

impl_oprf_for_uint!(name: FheUint2);
impl_oprf_for_uint!(name: FheUint4);
impl_oprf_for_uint!(name: FheUint6);
impl_oprf_for_uint!(name: FheUint8);
impl_oprf_for_uint!(name: FheUint10);
impl_oprf_for_uint!(name: FheUint12);
impl_oprf_for_uint!(name: FheUint14);
impl_oprf_for_uint!(name: FheUint16);
impl_oprf_for_uint!(name: FheUint32);
impl_oprf_for_uint!(name: FheUint64);
impl_oprf_for_uint!(name: FheUint128);
impl_oprf_for_uint!(name: FheUint160);
impl_oprf_for_uint!(name: FheUint256);
impl_oprf_for_uint!(name: FheUint512);
impl_oprf_for_uint!(name: FheUint1024);
impl_oprf_for_uint!(name: FheUint2048);

impl_oprf_for_int!(name: FheInt2);
impl_oprf_for_int!(name: FheInt4);
impl_oprf_for_int!(name: FheInt6);
impl_oprf_for_int!(name: FheInt8);
impl_oprf_for_int!(name: FheInt10);
impl_oprf_for_int!(name: FheInt12);
impl_oprf_for_int!(name: FheInt14);
impl_oprf_for_int!(name: FheInt16);
impl_oprf_for_int!(name: FheInt32);
impl_oprf_for_int!(name: FheInt64);
impl_oprf_for_int!(name: FheInt128);
impl_oprf_for_int!(name: FheInt160);
impl_oprf_for_int!(name: FheInt256);
