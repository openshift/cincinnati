use super::platform;
use super::UChar;
use std;
use std::char;
use std::ffi::{OsStr, OsString};
use std::mem;

/// An owned, mutable "wide" string for FFI that is **not** nul-aware.
///
/// `UString` is not aware of nul values. Strings may or may not be nul-terminated, and may
/// contain invalid and ill-formed UTF-16 or UTF-32 data. These strings are intended to be used
/// with FFI functions that directly use string length, where the strings are known to have proper
/// nul-termination already, or where strings are merely being passed through without modification.
///
/// `UCString` should be used instead if nul-aware strings are required.
///
/// `UString` can be converted to and from many other standard Rust string types, including
/// `OsString` and `String`, making proper Unicode FFI safe and easy.
///
/// Please prefer using the type aliases `U16String` or `U32String` or `WideString` to using this
/// type directly.
///
/// # Examples
///
/// The following example constructs a `U16String` and shows how to convert a `U16String` to a
/// regular Rust `String`.
///
/// ```rust
/// use widestring::U16String;
/// let s = "Test";
/// // Create a wide string from the rust string
/// let wstr = U16String::from_str(s);
/// // Convert back to a rust string
/// let rust_str = wstr.to_string_lossy();
/// assert_eq!(rust_str, "Test");
/// ```
///
/// The same example using `U32String` instead:
///
/// ```rust
/// use widestring::U32String;
/// let s = "Test";
/// // Create a wide string from the rust string
/// let wstr = U32String::from_str(s);
/// // Convert back to a rust string
/// let rust_str = wstr.to_string_lossy();
/// assert_eq!(rust_str, "Test");
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UString<C: UChar> {
    inner: Vec<C>,
}

/// String slice reference for `U16String`.
///
/// `UStr` is to `UString` as `str` is to `String`.
///
/// `UStr` is not aware of nul values. Strings may or may not be nul-terminated, and may
/// contain invalid and ill-formed UTF-16 or UTF-32 data. These strings are intended to be used
/// with FFI functions that directly use string length, where the strings are known to have proper
/// nul-termination already, or where strings are merely being passed through without modification.
///
/// `UCStr` should be used instead of nul-aware strings are required.
///
/// `UStr` can be converted to many other string types, including `OsString` and `String`, making
/// proper Unicode FFI safe and easy.
///
/// Please prefer using the type aliases `U16Str` or `U32Str` or `WideStr` to using this type
/// directly.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UStr<C: UChar> {
    inner: [C],
}

/// A possible error value when converting a String from a UTF-32 byte slice.
///
/// This type is the error type for the `to_string` method on `U32Str`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FromUtf32Error();

impl<C: UChar> UString<C> {
    /// Constructs a new empty `UString`.
    pub fn new() -> Self {
        Self { inner: vec![] }
    }

    /// Constructs a `UString` from a vector of possibly invalid or ill-formed UTF-16 or UTF-32
    /// data.
    ///
    /// No checks are made on the contents of the vector.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U16String;
    /// let v = vec![84u16, 104u16, 101u16]; // 'T' 'h' 'e'
    /// # let cloned = v.clone();
    /// // Create a wide string from the vector
    /// let wstr = U16String::from_vec(v);
    /// # assert_eq!(wstr.into_vec(), cloned);
    /// ```
    ///
    /// ```rust
    /// use widestring::U32String;
    /// let v = vec![84u32, 104u32, 101u32]; // 'T' 'h' 'e'
    /// # let cloned = v.clone();
    /// // Create a wide string from the vector
    /// let wstr = U32String::from_vec(v);
    /// # assert_eq!(wstr.into_vec(), cloned);
    /// ```
    pub fn from_vec(raw: impl Into<Vec<C>>) -> Self {
        Self { inner: raw.into() }
    }

    /// Constructs a `UString` from a pointer and a length.
    ///
    /// The `len` argument is the number of elements, **not** the number of bytes.
    ///
    /// # Safety
    ///
    /// This function is unsafe as there is no guarantee that the given pointer is valid for `len`
    /// elements.
    ///
    /// # Panics
    ///
    /// Panics if `len` is greater than 0 but `p` is a null pointer.
    pub unsafe fn from_ptr(p: *const C, len: usize) -> Self {
        if len == 0 {
            return Self::new();
        }
        assert!(!p.is_null());
        let slice = std::slice::from_raw_parts(p, len);
        Self::from_vec(slice)
    }

    /// Creates a `UString` with the given capacity.
    ///
    /// The string will be able to hold exactly `capacity` partial code units without reallocating.
    /// If `capacity` is set to 0, the string will not initially allocate.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
        }
    }

    /// Returns the capacity this `UString` can hold without reallocating.
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Truncate the `UString` to zero length.
    pub fn clear(&mut self) {
        self.inner.clear()
    }

    /// Reserves the capacity for at least `additional` more capacity to be inserted in the given
    /// `UString`.
    ///
    /// More space may be reserved to avoid frequent allocations.
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional)
    }

    /// Reserves the minimum capacity for exactly `additional` more capacity to be inserted in the
    /// given `UString`. Does nothing if the capcity is already sufficient.
    ///
    /// Note that the allocator may give more space than is requested. Therefore capacity can not
    /// be relied upon to be precisely minimal. Prefer `reserve` if future insertions are expected.
    pub fn reserve_exact(&mut self, additional: usize) {
        self.inner.reserve_exact(additional)
    }

    /// Converts the wide string into a `Vec`, consuming the string in the process.
    pub fn into_vec(self) -> Vec<C> {
        self.inner
    }

    /// Converts to a `UStr` reference.
    pub fn as_ustr(&self) -> &UStr<C> {
        self
    }

    /// Extends the wide string with the given `&UStr`.
    ///
    /// No checks are performed on the strings. It is possible to end up nul values inside the
    /// string, and it is up to the caller to determine if that is acceptable.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U16String;
    /// let s = "MyString";
    /// let mut wstr = U16String::from_str(s);
    /// let cloned = wstr.clone();
    /// // Push the clone to the end, repeating the string twice.
    /// wstr.push(cloned);
    ///
    /// assert_eq!(wstr.to_string().unwrap(), "MyStringMyString");
    /// ```
    ///
    /// ```rust
    /// use widestring::U32String;
    /// let s = "MyString";
    /// let mut wstr = U32String::from_str(s);
    /// let cloned = wstr.clone();
    /// // Push the clone to the end, repeating the string twice.
    /// wstr.push(cloned);
    ///
    /// assert_eq!(wstr.to_string().unwrap(), "MyStringMyString");
    /// ```
    pub fn push(&mut self, s: impl AsRef<UStr<C>>) {
        self.inner.extend_from_slice(&s.as_ref().inner)
    }

    /// Extends the wide string with the given slice.
    ///
    /// No checks are performed on the strings. It is possible to end up nul values inside the
    /// string, and it is up to the caller to determine if that is acceptable.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U16String;
    /// let s = "MyString";
    /// let mut wstr = U16String::from_str(s);
    /// let cloned = wstr.clone();
    /// // Push the clone to the end, repeating the string twice.
    /// wstr.push_slice(cloned);
    ///
    /// assert_eq!(wstr.to_string().unwrap(), "MyStringMyString");
    /// ```
    ///
    /// ```rust
    /// use widestring::U32String;
    /// let s = "MyString";
    /// let mut wstr = U32String::from_str(s);
    /// let cloned = wstr.clone();
    /// // Push the clone to the end, repeating the string twice.
    /// wstr.push_slice(cloned);
    ///
    /// assert_eq!(wstr.to_string().unwrap(), "MyStringMyString");
    /// ```
    pub fn push_slice(&mut self, s: impl AsRef<[C]>) {
        self.inner.extend_from_slice(&s.as_ref())
    }

    /// Shrinks the capacity of the `UString` to match its length.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U16String;
    ///
    /// let mut s = U16String::from_str("foo");
    ///
    /// s.reserve(100);
    /// assert!(s.capacity() >= 100);
    ///
    /// s.shrink_to_fit();
    /// assert_eq!(3, s.capacity());
    /// ```
    ///
    /// ```rust
    /// use widestring::U32String;
    ///
    /// let mut s = U32String::from_str("foo");
    ///
    /// s.reserve(100);
    /// assert!(s.capacity() >= 100);
    ///
    /// s.shrink_to_fit();
    /// assert_eq!(3, s.capacity());
    /// ```
    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
    }

    /// Converts this `UString` into a boxed `UStr`.
    ///
    /// # Examples
    ///
    /// ```
    /// use widestring::{U16String, U16Str};
    ///
    /// let s = U16String::from_str("hello");
    ///
    /// let b: Box<U16Str> = s.into_boxed_ustr();
    /// ```
    ///
    /// ```
    /// use widestring::{U32String, U32Str};
    ///
    /// let s = U32String::from_str("hello");
    ///
    /// let b: Box<U32Str> = s.into_boxed_ustr();
    /// ```
    pub fn into_boxed_ustr(self) -> Box<UStr<C>> {
        let rw = Box::into_raw(self.inner.into_boxed_slice()) as *mut UStr<C>;
        unsafe { Box::from_raw(rw) }
    }
}

impl UString<u16> {
    /// Encodes a `U16String` copy from a `str`.
    ///
    /// This makes a wide string copy of the `str`. Since `str` will always be valid UTF-8, the
    /// resulting `U16String` will also be valid UTF-16.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U16String;
    /// let s = "MyString";
    /// // Create a wide string from the string
    /// let wstr = U16String::from_str(s);
    ///
    /// assert_eq!(wstr.to_string().unwrap(), s);
    /// ```
    pub fn from_str<S: AsRef<str> + ?Sized>(s: &S) -> Self {
        Self {
            inner: s.as_ref().encode_utf16().collect(),
        }
    }

    /// Encodes a `U16String` copy from an `OsStr`.
    ///
    /// This makes a wide string copy of the `OsStr`. Since `OsStr` makes no guarantees that it is
    /// valid data, there is no guarantee that the resulting `U16String` will be valid UTF-16.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U16String;
    /// let s = "MyString";
    /// // Create a wide string from the string
    /// let wstr = U16String::from_os_str(s);
    ///
    /// assert_eq!(wstr.to_string().unwrap(), s);
    /// ```
    pub fn from_os_str<S: AsRef<OsStr> + ?Sized>(s: &S) -> Self {
        Self {
            inner: platform::os_to_wide(s.as_ref()),
        }
    }

    /// Extends the string with the given `&str`.
    ///
    /// No checks are performed on the strings. It is possible to end up nul values inside the
    /// string, and it is up to the caller to determine if that is acceptable.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U16String;
    /// let s = "MyString";
    /// let mut wstr = U16String::from_str(s);
    /// // Push the original to the end, repeating the string twice.
    /// wstr.push_str(s);
    ///
    /// assert_eq!(wstr.to_string().unwrap(), "MyStringMyString");
    /// ```
    pub fn push_str(&mut self, s: impl AsRef<str>) {
        self.inner.extend(s.as_ref().encode_utf16())
    }

    /// Extends the string with the given `&OsStr`.
    ///
    /// No checks are performed on the strings. It is possible to end up nul values inside the
    /// string, and it is up to the caller to determine if that is acceptable.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U16String;
    /// let s = "MyString";
    /// let mut wstr = U16String::from_str(s);
    /// // Push the original to the end, repeating the string twice.
    /// wstr.push_os_str(s);
    ///
    /// assert_eq!(wstr.to_string().unwrap(), "MyStringMyString");
    /// ```
    pub fn push_os_str(&mut self, s: impl AsRef<OsStr>) {
        self.inner.extend(platform::os_to_wide(s.as_ref()))
    }
}

impl UString<u32> {
    /// Constructs a `U32String` from a vector of UTF-32 data.
    ///
    /// No checks are made on the contents of the vector.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U32String;
    /// let v: Vec<char> = "Test".chars().collect();
    /// # let cloned: Vec<u32> = v.iter().map(|&c| c as u32).collect();
    /// // Create a wide string from the vector
    /// let wstr = U32String::from_chars(v);
    /// # assert_eq!(wstr.into_vec(), cloned);
    /// ```
    pub fn from_chars(raw: impl Into<Vec<char>>) -> Self {
        UString {
            inner: unsafe { mem::transmute(raw.into()) },
        }
    }

    /// Encodes a `U32String` copy from a `str`.
    ///
    /// This makes a wide string copy of the `str`. Since `str` will always be valid UTF-8, the
    /// resulting `U32String` will also be valid UTF-32.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U32String;
    /// let s = "MyString";
    /// // Create a wide string from the string
    /// let wstr = U32String::from_str(s);
    ///
    /// assert_eq!(wstr.to_string().unwrap(), s);
    /// ```
    pub fn from_str<S: AsRef<str> + ?Sized>(s: &S) -> Self {
        let v: Vec<char> = s.as_ref().chars().collect();
        UString::from_chars(v)
    }

    /// Encodes a `U32String` copy from an `OsStr`.
    ///
    /// This makes a wide string copy of the `OsStr`. Since `OsStr` makes no guarantees that it is
    /// valid data, there is no guarantee that the resulting `U32String` will be valid UTF-32.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U32String;
    /// let s = "MyString";
    /// // Create a wide string from the string
    /// let wstr = U32String::from_os_str(s);
    ///
    /// assert_eq!(wstr.to_string().unwrap(), s);
    /// ```
    pub fn from_os_str<S: AsRef<OsStr> + ?Sized>(s: &S) -> Self {
        let v: Vec<char> = s.as_ref().to_string_lossy().chars().collect();
        UString::from_chars(v)
    }

    /// Constructs a `U32String` from a `char` pointer and a length.
    ///
    /// The `len` argument is the number of `char` elements, **not** the number of bytes.
    ///
    /// # Safety
    ///
    /// This function is unsafe as there is no guarantee that the given pointer is valid for `len`
    /// elements.
    ///
    /// # Panics
    ///
    /// Panics if `len` is greater than 0 but `p` is a null pointer.
    pub unsafe fn from_char_ptr(p: *const char, len: usize) -> Self {
        UString::from_ptr(p as *const u32, len)
    }

    /// Extends the string with the given `&str`.
    ///
    /// No checks are performed on the strings. It is possible to end up nul values inside the
    /// string, and it is up to the caller to determine if that is acceptable.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U32String;
    /// let s = "MyString";
    /// let mut wstr = U32String::from_str(s);
    /// // Push the original to the end, repeating the string twice.
    /// wstr.push_str(s);
    ///
    /// assert_eq!(wstr.to_string().unwrap(), "MyStringMyString");
    /// ```
    pub fn push_str(&mut self, s: impl AsRef<str>) {
        self.inner.extend(s.as_ref().chars().map(|c| c as u32))
    }

    /// Extends the string with the given `&OsStr`.
    ///
    /// No checks are performed on the strings. It is possible to end up nul values inside the
    /// string, and it is up to the caller to determine if that is acceptable.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U32String;
    /// let s = "MyString";
    /// let mut wstr = U32String::from_str(s);
    /// // Push the original to the end, repeating the string twice.
    /// wstr.push_os_str(s);
    ///
    /// assert_eq!(wstr.to_string().unwrap(), "MyStringMyString");
    /// ```
    pub fn push_os_str(&mut self, s: impl AsRef<OsStr>) {
        self.inner
            .extend(s.as_ref().to_string_lossy().chars().map(|c| c as u32))
    }
}

impl<C: UChar> UStr<C> {
    /// Coerces a value into a `UStr`.
    pub fn new<S: AsRef<Self> + ?Sized>(s: &S) -> &Self {
        s.as_ref()
    }

    /// Constructs a `UStr` from a pointer and a length.
    ///
    /// The `len` argument is the number of elements, **not** the number of bytes.
    ///
    /// # Safety
    ///
    /// This function is unsafe as there is no guarantee that the given pointer is valid for `len`
    /// elements.
    ///
    /// # Panics
    ///
    /// This function panics if `p` is null.
    ///
    /// # Caveat
    ///
    /// The lifetime for the returned string is inferred from its usage. To prevent accidental
    /// misuse, it's suggested to tie the lifetime to whichever source lifetime is safe in the
    /// context, such as by providing a helper function taking the lifetime of a host value for the
    /// string, or by explicit annotation.
    pub unsafe fn from_ptr<'a>(p: *const C, len: usize) -> &'a Self {
        assert!(!p.is_null());
        mem::transmute(std::slice::from_raw_parts(p, len))
    }

    /// Constructs a `UStr` from a slice of code points.
    ///
    /// No checks are performed on the slice.
    pub fn from_slice(slice: &[C]) -> &Self {
        unsafe { mem::transmute(slice) }
    }

    /// Copies the wide string to a new owned `UString`.
    pub fn to_ustring(&self) -> UString<C> {
        UString::from_vec(&self.inner)
    }

    /// Converts to a slice of the wide string.
    pub fn as_slice(&self) -> &[C] {
        &self.inner
    }

    /// Returns a raw pointer to the wide string.
    ///
    /// The pointer is valid only as long as the lifetime of this reference.
    pub fn as_ptr(&self) -> *const C {
        self.inner.as_ptr()
    }

    /// Returns the length of the wide string as number of elements (**not** number of bytes).
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether this wide string contains no data.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Converts a `Box<UStr>` into a `UString` without copying or allocating.
    pub fn into_ustring(self: Box<Self>) -> UString<C> {
        let boxed = unsafe { Box::from_raw(Box::into_raw(self) as *mut [C]) };
        UString {
            inner: boxed.into_vec(),
        }
    }
}

impl UStr<u16> {
    /// Decodes a wide string to an owned `OsString`.
    ///
    /// This makes a string copy of the `U16Str`. Since `U16Str` makes no guarantees that it is
    /// valid UTF-16, there is no guarantee that the resulting `OsString` will be valid data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U16String;
    /// use std::ffi::OsString;
    /// let s = "MyString";
    /// // Create a wide string from the string
    /// let wstr = U16String::from_str(s);
    /// // Create an OsString from the wide string
    /// let osstr = wstr.to_os_string();
    ///
    /// assert_eq!(osstr, OsString::from(s));
    /// ```
    pub fn to_os_string(&self) -> OsString {
        platform::os_from_wide(&self.inner)
    }

    /// Copies the wide string to a `String` if it contains valid UTF-16 data.
    ///
    /// # Failures
    ///
    /// Returns an error if the string contains any invalid UTF-16 data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U16String;
    /// let s = "MyString";
    /// // Create a wide string from the string
    /// let wstr = U16String::from_str(s);
    /// // Create a regular string from the wide string
    /// let s2 = wstr.to_string().unwrap();
    ///
    /// assert_eq!(s2, s);
    /// ```
    pub fn to_string(&self) -> Result<String, std::string::FromUtf16Error> {
        String::from_utf16(&self.inner)
    }

    /// Copies the wide string to a `String`.
    ///
    /// Any non-Unicode sequences are replaced with *U+FFFD REPLACEMENT CHARACTER*.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U16String;
    /// let s = "MyString";
    /// // Create a wide string from the string
    /// let wstr = U16String::from_str(s);
    /// // Create a regular string from the wide string
    /// let lossy = wstr.to_string_lossy();
    ///
    /// assert_eq!(lossy, s);
    /// ```
    pub fn to_string_lossy(&self) -> String {
        String::from_utf16_lossy(&self.inner)
    }
}

impl UStr<u32> {
    /// Constructs a `U32Str` from a `char` pointer and a length.
    ///
    /// The `len` argument is the number of `char` elements, **not** the number of bytes.
    ///
    /// # Safety
    ///
    /// This function is unsafe as there is no guarantee that the given pointer is valid for `len`
    /// elements.
    ///
    /// # Panics
    ///
    /// This function panics if `p` is null.
    ///
    /// # Caveat
    ///
    /// The lifetime for the returned string is inferred from its usage. To prevent accidental
    /// misuse, it's suggested to tie the lifetime to whichever source lifetime is safe in the
    /// context, such as by providing a helper function taking the lifetime of a host value for the
    /// string, or by explicit annotation.
    pub unsafe fn from_char_ptr<'a>(p: *const char, len: usize) -> &'a Self {
        UStr::from_ptr(p as *const u32, len)
    }

    /// Constructs a `U32Str` from a slice of `u32` code points.
    ///
    /// No checks are performed on the slice.
    pub fn from_char_slice(slice: &[char]) -> &Self {
        unsafe { mem::transmute(slice) }
    }

    /// Decodes a wide string to an owned `OsString`.
    ///
    /// This makes a string copy of the `U32Str`. Since `U32Str` makes no guarantees that it is
    /// valid UTF-32, there is no guarantee that the resulting `OsString` will be valid data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U32String;
    /// use std::ffi::OsString;
    /// let s = "MyString";
    /// // Create a wide string from the string
    /// let wstr = U32String::from_str(s);
    /// // Create an OsString from the wide string
    /// let osstr = wstr.to_os_string();
    ///
    /// assert_eq!(osstr, OsString::from(s));
    /// ```
    pub fn to_os_string(&self) -> OsString {
        self.to_string_lossy().into()
    }

    /// Copies the wide string to a `String` if it contains valid UTF-32 data.
    ///
    /// # Failures
    ///
    /// Returns an error if the string contains any invalid UTF-32 data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U32String;
    /// let s = "MyString";
    /// // Create a wide string from the string
    /// let wstr = U32String::from_str(s);
    /// // Create a regular string from the wide string
    /// let s2 = wstr.to_string().unwrap();
    ///
    /// assert_eq!(s2, s);
    /// ```
    pub fn to_string(&self) -> Result<String, FromUtf32Error> {
        let chars: Vec<Option<char>> = self.inner.iter().map(|c| char::from_u32(*c)).collect();
        if chars.iter().any(|c| c.is_none()) {
            return Err(FromUtf32Error());
        }
        let size = chars.iter().filter_map(|o| o.map(|c| c.len_utf8())).sum();
        let mut vec = Vec::with_capacity(size);
        unsafe { vec.set_len(size) };
        let mut i = 0;
        for c in chars.iter().filter_map(|&o| o) {
            c.encode_utf8(&mut vec[i..]);
            i += c.len_utf8();
        }
        Ok(unsafe { String::from_utf8_unchecked(vec) })
    }

    /// Copies the wide string to a `String`.
    ///
    /// Any non-Unicode sequences are replaced with *U+FFFD REPLACEMENT CHARACTER*.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use widestring::U32String;
    /// let s = "MyString";
    /// // Create a wide string from the string
    /// let wstr = U32String::from_str(s);
    /// // Create a regular string from the wide string
    /// let lossy = wstr.to_string_lossy();
    ///
    /// assert_eq!(lossy, s);
    /// ```
    pub fn to_string_lossy(&self) -> String {
        let chars: Vec<char> = self
            .inner
            .iter()
            .map(|&c| char::from_u32(c).unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect();
        let size = chars.iter().map(|c| c.len_utf8()).sum();
        let mut vec = Vec::with_capacity(size);
        unsafe { vec.set_len(size) };
        let mut i = 0;
        for c in chars {
            c.encode_utf8(&mut vec[i..]);
            i += c.len_utf8();
        }
        unsafe { String::from_utf8_unchecked(vec) }
    }
}

impl<C: UChar> Into<Vec<C>> for UString<C> {
    fn into(self) -> Vec<C> {
        self.into_vec()
    }
}

impl<'a> From<UString<u16>> for std::borrow::Cow<'a, UStr<u16>> {
    fn from(s: UString<u16>) -> Self {
        std::borrow::Cow::Owned(s)
    }
}

impl<'a> From<UString<u32>> for std::borrow::Cow<'a, UStr<u32>> {
    fn from(s: UString<u32>) -> Self {
        std::borrow::Cow::Owned(s)
    }
}

impl Into<UString<u16>> for Vec<u16> {
    fn into(self) -> UString<u16> {
        UString::from_vec(self)
    }
}

impl Into<UString<u32>> for Vec<u32> {
    fn into(self) -> UString<u32> {
        UString::from_vec(self)
    }
}

impl Into<UString<u32>> for Vec<char> {
    fn into(self) -> UString<u32> {
        UString::from_chars(self)
    }
}

impl From<String> for UString<u16> {
    fn from(s: String) -> Self {
        Self::from_str(&s)
    }
}

impl From<String> for UString<u32> {
    fn from(s: String) -> Self {
        Self::from_str(&s)
    }
}

impl From<OsString> for UString<u16> {
    fn from(s: OsString) -> Self {
        Self::from_os_str(&s)
    }
}

impl From<OsString> for UString<u32> {
    fn from(s: OsString) -> Self {
        Self::from_os_str(&s)
    }
}

impl From<UString<u16>> for OsString {
    fn from(s: UString<u16>) -> Self {
        s.to_os_string()
    }
}

impl From<UString<u32>> for OsString {
    fn from(s: UString<u32>) -> Self {
        s.to_os_string()
    }
}

impl<'a, C: UChar, T: ?Sized + AsRef<UStr<C>>> From<&'a T> for UString<C> {
    fn from(s: &'a T) -> Self {
        s.as_ref().to_ustring()
    }
}

impl<C: UChar> std::ops::Index<std::ops::RangeFull> for UString<C> {
    type Output = UStr<C>;

    #[inline]
    fn index(&self, _index: std::ops::RangeFull) -> &UStr<C> {
        UStr::from_slice(&self.inner)
    }
}

impl<C: UChar> std::ops::Deref for UString<C> {
    type Target = UStr<C>;

    #[inline]
    fn deref(&self) -> &UStr<C> {
        &self[..]
    }
}

impl<C: UChar> PartialEq<UStr<C>> for UString<C> {
    #[inline]
    fn eq(&self, other: &UStr<C>) -> bool {
        self.as_ustr() == other
    }
}

impl<C: UChar> PartialOrd<UStr<C>> for UString<C> {
    #[inline]
    fn partial_cmp(&self, other: &UStr<C>) -> Option<std::cmp::Ordering> {
        self.as_ustr().partial_cmp(other)
    }
}

impl<'a, C: UChar> PartialEq<&'a UStr<C>> for UString<C> {
    #[inline]
    fn eq(&self, other: &&'a UStr<C>) -> bool {
        self.as_ustr() == *other
    }
}

impl<'a, C: UChar> PartialOrd<&'a UStr<C>> for UString<C> {
    #[inline]
    fn partial_cmp(&self, other: &&'a UStr<C>) -> Option<std::cmp::Ordering> {
        self.as_ustr().partial_cmp(*other)
    }
}

impl<'a, C: UChar> PartialEq<std::borrow::Cow<'a, UStr<C>>> for UString<C> {
    #[inline]
    fn eq(&self, other: &std::borrow::Cow<'a, UStr<C>>) -> bool {
        self.as_ustr() == other.as_ref()
    }
}

impl<'a, C: UChar> PartialOrd<std::borrow::Cow<'a, UStr<C>>> for UString<C> {
    #[inline]
    fn partial_cmp(&self, other: &std::borrow::Cow<'a, UStr<C>>) -> Option<std::cmp::Ordering> {
        self.as_ustr().partial_cmp(other.as_ref())
    }
}

impl<C: UChar> std::borrow::Borrow<UStr<C>> for UString<C> {
    fn borrow(&self) -> &UStr<C> {
        &self[..]
    }
}

impl<C: UChar> ToOwned for UStr<C> {
    type Owned = UString<C>;
    fn to_owned(&self) -> UString<C> {
        self.to_ustring()
    }
}

impl<'a> From<&'a UStr<u16>> for std::borrow::Cow<'a, UStr<u16>> {
    fn from(s: &'a UStr<u16>) -> Self {
        std::borrow::Cow::Borrowed(s)
    }
}

impl<'a> From<&'a UStr<u32>> for std::borrow::Cow<'a, UStr<u32>> {
    fn from(s: &'a UStr<u32>) -> Self {
        std::borrow::Cow::Borrowed(s)
    }
}

impl<C: UChar> AsRef<UStr<C>> for UStr<C> {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<C: UChar> AsRef<UStr<C>> for UString<C> {
    fn as_ref(&self) -> &UStr<C> {
        self
    }
}

impl<C: UChar> AsRef<[C]> for UStr<C> {
    fn as_ref(&self) -> &[C] {
        self.as_slice()
    }
}

impl<C: UChar> AsRef<[C]> for UString<C> {
    fn as_ref(&self) -> &[C] {
        self.as_slice()
    }
}

impl<'a, C: UChar> From<&'a UStr<C>> for Box<UStr<C>> {
    fn from(s: &'a UStr<C>) -> Self {
        let boxed: Box<[C]> = Box::from(&s.inner);
        let rw = Box::into_raw(boxed) as *mut UStr<C>;
        unsafe { Box::from_raw(rw) }
    }
}

impl<C: UChar> From<Box<UStr<C>>> for UString<C> {
    fn from(boxed: Box<UStr<C>>) -> Self {
        boxed.into_ustring()
    }
}

impl<C: UChar> From<UString<C>> for Box<UStr<C>> {
    fn from(s: UString<C>) -> Self {
        s.into_boxed_ustr()
    }
}

impl<C: UChar> Default for Box<UStr<C>> {
    fn default() -> Self {
        let boxed: Box<[C]> = Box::from([]);
        let rw = Box::into_raw(boxed) as *mut UStr<C>;
        unsafe { Box::from_raw(rw) }
    }
}

impl std::fmt::Display for FromUtf32Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "error converting from UTF-32 to UTF-8")
    }
}

impl std::error::Error for FromUtf32Error {
    fn description(&self) -> &str {
        "error converting from UTF-32 to UTF-8"
    }
}
