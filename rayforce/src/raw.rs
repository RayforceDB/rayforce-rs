//! Thin, `unsafe` accessors over the bindgen-generated `ray_t` layout.
//!
//! The C header exposes the hot-path field reads as function-like macros
//! (`ray_type`, `ray_len`, `ray_data`, `RAY_IS_ERR`, …) which bindgen does not
//! emit. We reimplement them here against the generated union so the rest of the
//! safe crate never touches `__bindgen_anon_*` directly. Field access goes
//! through bindgen's `__BindgenUnionField`/real-union types, so these stay in
//! sync with the generated layout automatically.

use rayforce_sys as sys;

/// Raw, non-owning pointer to a core object.
pub(crate) type Raw = *mut sys::ray_t;

/// The 32-byte allocated-object header view (`ray_t`'s first union arm).
#[inline]
pub(crate) unsafe fn hdr<'a>(v: Raw) -> &'a sys::ray_t__bindgen_ty_1 {
    (*v).__bindgen_anon_1.as_ref()
}

/// The value-payload union (bytes 24..32): scalar value, `obj`, SSO string, or
/// vector length.
#[inline]
unsafe fn val<'a>(v: Raw) -> &'a sys::ray_t__bindgen_ty_1__bindgen_ty_2 {
    &hdr(v).__bindgen_anon_2
}

#[inline]
pub(crate) unsafe fn type_code(v: Raw) -> i8 {
    hdr(v).type_
}

#[inline]
pub(crate) unsafe fn attrs(v: Raw) -> u8 {
    hdr(v).attrs
}

/// Overwrite the attribute byte. Used to turn a quoted symbol literal into a
/// name reference (clear `ATTR_QUOTED`) for the query compiler.
#[inline]
pub(crate) unsafe fn set_attrs(v: Raw, a: u8) {
    (*v).__bindgen_anon_1.as_mut().attrs = a;
}

/// Resolution domain of a `RAY_SYM` vector (mirrors the engine's static-inline
/// `ray_sym_vec_domain`, which bindgen does not emit): follow a slice to its
/// parent, read the `sym_domain` aux field, and fall back to the runtime domain
/// when unset. A `RAY_SYM` cell is a *position* in this domain — for FILE
/// domains (e.g. a loaded splayed column) positions are not runtime intern ids
/// and must be mapped via `ray_sym_domain_runtime_lut`.
#[inline]
pub(crate) unsafe fn sym_domain(v: Raw) -> *mut sys::ray_sym_domain_s {
    let h = hdr(v);
    if h.attrs & sys::RAY_ATTR_SLICE as u8 != 0 {
        let parent = h.__bindgen_anon_1.__bindgen_anon_1.slice_parent;
        if !parent.is_null() {
            return sym_domain(parent);
        }
    }
    let d = h.__bindgen_anon_1.__bindgen_anon_3.sym_domain;
    if d.is_null() {
        sys::ray_sym_runtime_domain()
    } else {
        d
    }
}

#[inline]
pub(crate) unsafe fn rc(v: Raw) -> u32 {
    hdr(v).rc
}

/// `|type|` — canonical type id (atoms are negative, vectors positive).
#[inline]
pub(crate) unsafe fn abs_type(v: Raw) -> i8 {
    let t = type_code(v);
    if t < 0 {
        -t
    } else {
        t
    }
}

#[inline]
pub(crate) unsafe fn is_atom(v: Raw) -> bool {
    let t = type_code(v);
    t < 0 || t >= sys::RAY_LAMBDA as i8
}

#[inline]
pub(crate) unsafe fn is_vec(v: Raw) -> bool {
    let t = type_code(v);
    t >= sys::RAY_BOOL as i8 && t <= sys::RAY_STR as i8
}

/// Vector / list element count (reads the `len` arm of the value union).
#[inline]
pub(crate) unsafe fn len(v: Raw) -> i64 {
    val(v).len
}

#[inline]
pub(crate) unsafe fn as_i64(v: Raw) -> i64 {
    val(v).i64_
}
#[inline]
pub(crate) unsafe fn as_i32(v: Raw) -> i32 {
    val(v).i32_
}
#[inline]
pub(crate) unsafe fn as_i16(v: Raw) -> i16 {
    val(v).i16_
}
#[inline]
pub(crate) unsafe fn as_u8(v: Raw) -> u8 {
    val(v).u8_
}
#[inline]
pub(crate) unsafe fn as_b8(v: Raw) -> bool {
    val(v).b8 != 0
}
#[inline]
pub(crate) unsafe fn as_f64(v: Raw) -> f64 {
    val(v).f64_
}
/// Child object pointer (long strings / GUID payload).
#[inline]
pub(crate) unsafe fn child_obj(v: Raw) -> Raw {
    val(v).obj
}

/// Pointer to the element data of a vector, transparently following slices.
#[inline]
pub(crate) unsafe fn data(v: Raw) -> *mut u8 {
    if attrs(v) & sys::RAY_ATTR_SLICE as u8 != 0 {
        sys::ray_data_slice_path(v) as *mut u8
    } else {
        hdr(v).data.as_ptr() as *mut u8
    }
}

/// The untyped null singleton (`RAY_NULL_OBJ`).
#[inline]
pub(crate) fn null_obj() -> Raw {
    std::ptr::addr_of_mut!(sys::__ray_null)
}

#[inline]
pub(crate) fn is_null_singleton(v: Raw) -> bool {
    v == null_obj()
}

/// True if `v` is a `RAY_ERROR` object. Mirrors the header's `RAY_IS_ERR`.
#[inline]
pub(crate) unsafe fn is_err(v: Raw) -> bool {
    !v.is_null() && (v as usize) > 31 && type_code(v) == sys::RAY_ERROR as i8
}

/// SSO string length byte (valid only for `-RAY_STR` atoms).
#[inline]
unsafe fn sso_len(v: Raw) -> u8 {
    val(v).__bindgen_anon_1.slen
}

/// Copy the 16-byte GUID payload of a `-RAY_GUID` atom, if present.
///
/// Returns by value (not a borrow) so the bytes can't outlive the owning
/// object: the payload lives in a child block that is reclaimed when the last
/// reference drops.
#[inline]
pub(crate) unsafe fn guid_bytes(v: Raw) -> Option<[u8; 16]> {
    let obj = child_obj(v);
    if obj.is_null() {
        return None;
    }
    Some(*(data(obj) as *const [u8; 16]))
}

/// Atom null predicate. Reimplements the header's inline `ray_atom_is_null_fn`
/// against the per-type sentinel encoding.
#[inline]
pub(crate) unsafe fn atom_is_null(v: Raw) -> bool {
    if is_null_singleton(v) {
        return true;
    }
    let t = type_code(v);
    if t >= 0 {
        return false;
    }
    match (-t) as u32 {
        sys::RAY_F64 => as_f64(v).is_nan(),
        sys::RAY_F32 => (as_f64(v) as f32).is_nan(),
        sys::RAY_I64 | sys::RAY_TIMESTAMP => as_i64(v) == i64::MIN,
        sys::RAY_I32 | sys::RAY_DATE | sys::RAY_TIME => as_i32(v) == i32::MIN,
        sys::RAY_I16 => as_i16(v) == i16::MIN,
        sys::RAY_SYM => as_i64(v) == 0,
        sys::RAY_STR => sso_len(v) == 0 && child_obj(v).is_null(),
        sys::RAY_GUID => match guid_bytes(v) {
            None => true,
            Some(b) => b.iter().all(|&x| x == 0),
        },
        // BOOL/U8 and anything else: the aux[0] low bit set by ray_typed_null.
        _ => hdr(v).__bindgen_anon_1.aux[0] & 1 != 0,
    }
}
