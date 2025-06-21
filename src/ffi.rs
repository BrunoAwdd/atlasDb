use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr;

use crate::{AtlasEnv, Proposal, ConsensusResult};

#[repr(C)]
pub struct AtlasEnvHandle {
    env: AtlasEnv,
}

/// Cria um novo ambiente Atlas com IDs fixos temporariamente
#[no_mangle]
pub extern "C" fn atlas_env_new() -> *mut AtlasEnvHandle {
    let env = AtlasEnv::new(&["A", "B", "C"]);
    Box::into_raw(Box::new(AtlasEnvHandle { env }))
}

#[no_mangle]
pub extern "C" fn atlas_env_free(ptr: *mut AtlasEnvHandle) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

#[no_mangle]
pub extern "C" fn atlas_env_submit_json_proposal(
    ptr: *mut AtlasEnvHandle,
    proposer: *const c_char,
    json: *const c_char,
) -> *mut c_void {
    if ptr.is_null() || proposer.is_null() || json.is_null() {
        return ptr::null_mut();
    }

    let env = unsafe { &mut *ptr };
    let proposer_str = unsafe { CStr::from_ptr(proposer).to_string_lossy() };
    let json_str = unsafe { CStr::from_ptr(json).to_string_lossy() };

    let json_value: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(val) => val,
        Err(_) => return ptr::null_mut(),
    };

    let proposal = env.env.submit_json_proposal(&proposer_str, json_value);
    Box::into_raw(Box::new(proposal)) as *mut c_void
}

#[no_mangle]
pub extern "C" fn atlas_env_evaluate_all(ptr: *mut AtlasEnvHandle) {
    if ptr.is_null() {
        return;
    }

    let env = unsafe { &mut *ptr };
    env.env.evaluate_all();
}

#[no_mangle]
pub extern "C" fn atlas_env_apply_if_approved(
    ptr: *mut AtlasEnvHandle,
    proposal: *const Proposal,
    result: *const ConsensusResult,
) {
    if ptr.is_null() || proposal.is_null() || result.is_null() {
        return;
    }

    let env = unsafe { &mut *ptr };
    let proposal = unsafe { &*proposal };
    let result = unsafe { &*result };
    env.env.apply_if_approved(proposal, result);
}

#[no_mangle]
pub extern "C" fn atlas_env_export_audit(ptr: *const AtlasEnvHandle, path: *const c_char) {
    if ptr.is_null() || path.is_null() {
        return;
    }

    let env = unsafe { &*ptr };
    let path_str = unsafe { CStr::from_ptr(path).to_string_lossy() };
    env.env.export_audit(&path_str);
}

#[no_mangle]
pub extern "C" fn atlas_env_print(ptr: *const AtlasEnvHandle) {
    if ptr.is_null() {
        return;
    }

    let env = unsafe { &*ptr };
    env.env.print();
}
