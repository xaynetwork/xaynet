#[macro_export]
/// Define field accessor methods for a trait to be implemented on a corresponding structure.
///
/// # Example
///
/// Writing `define_trait_fields!(bytes, Vec<u8>);` will generate the following trait method
/// signatures:
/// ```text
/// /// Get a reference to the bytes field.
/// fn bytes(&self) -> &Vec<u8>;
///
/// /// Get a mutable reference to the bytes field.
/// fn bytes_mut(&mut self) -> &mut Vec<u8>;
/// ```
/// The argument-tuples can be repeated by delimiting them with a semicolon.
macro_rules! define_trait_fields {
    ($($name:ident, $type:ty);+ $(;)?) => {
        paste::item! {
            $(
                /// Get a reference to the $name field.
                fn $name(&self) -> &$type;

                /// Get a mutable reference to the $name field.
                fn [<$name _mut>](&mut self) -> &mut $type;

            )+
        }
    };
}

#[macro_export]
/// Derive field accessor methods for a trait implemented on a corresponding structure.
///
/// # Example
///
/// Writing `derive_trait_fields!(bytes, Vec<u8>);` will generate the following trait method for a
/// corresponding structure containing the field `bytes: Vec<u8>`:
/// ```text
/// /// Get a reference to the bytes field.
/// fn bytes(&self) -> &Vec<u8> {
///     &self.bytes
/// }
///
/// /// Get a mutable reference to the bytes field.
/// fn bytes_mut(&mut self) -> &mut Vec<u8> {
///     &mut self.bytes
/// }
/// ```
/// The argument-tuples can be repeated by delimiting them with a semicolon.
macro_rules! derive_trait_fields {
    ($($name:ident, $type:ty);+ $(;)?) => {
        paste::item! {
            $(
                /// Get a reference to the $name field.
                fn $name(&self) -> &$type {
                    &self.$name
                }

                /// Get a mutable reference to the $name field.
                fn [<$name _mut>](&mut self) -> &mut $type {
                    &mut self.$name
                }
            )+

        }
    };
}

#[macro_export]
/// Derive field accessor methods for a structure.
///
/// # Example
///
/// Writing `derive_struct_fields!(bytes, Vec<u8>);` will generate the following struct method for a
/// corresponding structure containing the field `bytes: Vec<u8>`:
/// ```text
/// /// Get a reference to the bytes field.
/// pub fn bytes(&self) -> &Vec<u8> {
///     &self.bytes
/// }
/// ```
/// The argument-tuples can be repeated by delimiting them with a semicolon.
macro_rules! derive_struct_fields {
    ($($name:ident, $type:ty);+ $(;)?) => {
        $(
            /// Get a reference to the $name field.
            pub fn $name(&self) -> &$type {
                &self.$name
            }
        )+

    };
}
