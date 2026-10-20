/// Sign bit (bit 63)
pub const SIGN_BIT: u64 = 0x8000_0000_0000_0000;

/// Exponent with all bits set to 1 + Quiet NaN bit (51)
pub const QNAN: u64 = 0x7ffc_0000_0000_0000;

/// NULL type tag.
const TAG_NULL: u64 = 1;

/// Boolean "false" tag.
const TAG_FALSE: u64 = 2;

/// Boolean "true" tag.
const TAG_TRUE: u64 = 3;

/// Full null value representation with NaN-boxing.
pub const NULL_VAL: u64 = QNAN | TAG_NULL;

/// Full "false" value representation with NaN-boxing.
pub const FALSE_VAL: u64 = QNAN | TAG_FALSE;

/// Full "true" value representation with NaN-boxing.
pub const TRUE_VAL: u64 = QNAN | TAG_TRUE;

/// Heap objects, pointers and handles tag.
pub const OBJ_TAG: u64 = SIGN_BIT | QNAN;

/// Bitmask to extract payload from NaN-boxed values.
pub const PAYLOAD_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Value(pub u64);

/// This section of the implementation provides a set of "constructor"
/// functions to assemble values from raw types using NaN-boxing.
impl Value {
    /// Creates a new, "null" value using NaN-boxing.
    pub const fn null() -> Self {
        Self(NULL_VAL)
    }

    /// Creates a new boolean value using NaN-boxing.
    pub const fn bool(val: bool) -> Self {
        if val { Self(TRUE_VAL) } else { Self(FALSE_VAL) }
    }

    /// Creates a new numeric value using NaN-boxing and number
    /// canonicalisation to avoid NaN collisions.
    pub fn number(num: f64) -> Self {
        if num.is_nan() {
            Self(0x7ff8_0000_0000_0000)
        } else {
            Self(num.to_bits())
        }
    }

    /// Creates a heap object reference from an identifier or a 48-bit pointer.
    #[inline]
    pub fn obj(payload: u64) -> Self {
        debug_assert!((payload & !PAYLOAD_MASK) == 0, "Payload exceeds 48 bits.");
        Self(OBJ_TAG | (payload & PAYLOAD_MASK))
    }
}

/// This section of the implementation provides functions that are used to perform
/// runtime type-checking of the current NaN-boxed value.
impl Value {
    /// Checks if the given value is a number.
    #[inline]
    pub const fn is_number(&self) -> bool {
        (self.0 & QNAN) != QNAN
    }

    /// Checks if the current value is null.
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == NULL_VAL
    }

    /// Checks if the current value is a boolean value.
    #[inline]
    pub const fn is_bool(&self) -> bool {
        (self.0 | 1) == TRUE_VAL
    }

    /// Checks if the current value is a heap object.
    #[inline]
    pub const fn is_obj(&self) -> bool {
        (self.0 & (SIGN_BIT | QNAN)) == OBJ_TAG
    }

    /// Checks if the current value is "truthy".
    /// All values that are NOT False and Null are considered "truthy".
    #[inline]
    pub const fn is_truthy(&self) -> bool {
        !(self.is_null() || (self.is_bool() && !self.as_bool()))
    }
}

/// This section of the implementation provides NaN-unboxing functions to
/// convert values back to their raw corresponding types.
///
/// Basic conversions from numbers, booleans and memory objects are implemented
/// as core features of the runtime.
impl Value {
    #[inline]
    pub const fn as_number(&self) -> f64 {
        debug_assert!(self.is_number(), "Value is not a number.");
        f64::from_bits(self.0)
    }

    #[inline]
    pub const fn as_bool(&self) -> bool {
        debug_assert!(self.is_bool(), "Value is not a boolean.");
        self.0 == TRUE_VAL
    }

    #[inline]
    pub const fn as_obj(&self) -> u64 {
        debug_assert!(self.is_obj(), "Value is not a memory object.");
        self.0 & PAYLOAD_MASK
    }
}

use std::fmt;

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        if self.is_number() && other.is_number() {
            self.as_number() == other.as_number()
        } else {
            self.0 == other.0
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_number() {
            write!(f, "{}", self.as_number())
        } else if self.is_null() {
            write!(f, "null")
        } else if self.is_bool() {
            write!(f, "{}", self.as_bool())
        } else if self.is_obj() {
            write!(f, "<obj 0x{:x}>", self.as_obj())
        } else {
            write!(f, "<unknown 0x{:x}>", self.0)
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Value({})", self)
    }
}
