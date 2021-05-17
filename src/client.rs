// Citadel C bindings library (libcitadel)
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.

use libc::{c_char, c_int};
use std::ffi::c_void;
use std::ptr;
use std::str::FromStr;

use citadel::{rpc, Client, Error};

use crate::error::*;
use crate::{TryAsStr, TryIntoRaw, TryIntoString};

pub struct citadel_client_t {
    opaque: *mut c_void,
    message: *const c_char,
    err_no: c_int,
}

#[node_bindgen]
impl citadel_client_t {
    #[node_bindgen]
    pub(crate) fn with(inner_client: Client) -> *mut Self {
        let client = citadel_client_t {
            opaque: Box::into_raw(Box::new(inner_client)) as *mut c_void,
            err_no: SUCCESS,
            message: ptr::null(),
        };
        Box::into_raw(Box::new(client))
    }

    #[node_bindgen]
    pub(crate) fn from_err(error: citadel::Error) -> *mut Self {
        let mut client = citadel_client_t {
            opaque: ptr::null_mut(),
            err_no: c_int::MAX,
            message: ptr::null(),
        };
        client.set_error(error);
        Box::into_raw(Box::new(client))
    }

    #[node_bindgen]
    pub(crate) fn from_custom_err(err_no: c_int, msg: &str) -> *mut Self {
        let mut client = citadel_client_t {
            opaque: ptr::null_mut(),
            err_no,
            message: ptr::null(),
        };
        client.set_error_details(err_no, msg);
        Box::into_raw(Box::new(client))
    }

    #[node_bindgen]
    pub(crate) fn from_raw(client: *mut Self) -> &'static mut Self {
        unsafe { client.as_mut() }.expect("Wrong Citadel client pointer")
    }

    #[node_bindgen]
    pub(crate) fn try_as_opaque(&mut self) -> Option<&mut Client> {
        if self.opaque.is_null() {
            self.set_error_no(ERRNO_UNINIT);
            return None;
        }
        let boxed = unsafe { Box::from_raw(self.opaque as *mut Client) };
        Some(Box::leak(boxed))
    }

    #[node_bindgen]
    fn drop_message(&mut self) -> bool {
        let status = (self.message as *mut c_char).try_into_string().is_some();
        self.message = ptr::null();
        status
    }

    #[node_bindgen]
    pub(crate) fn set_success(&mut self) {
        self.err_no = SUCCESS;
        self.drop_message();
    }

    #[node_bindgen]
    pub(crate) fn set_error_details(
        &mut self,
        err_no: c_int,
        msg: impl ToString,
    ) {
        self.err_no = err_no;
        self.drop_message();
        self.message = msg
            .to_string()
            .try_into_raw()
            .unwrap_or("unparsable failure message".as_ptr() as *const c_char);
    }

    #[node_bindgen]
    pub(crate) fn set_error_no(&mut self, err_no: c_int) {
        let message = match err_no {
            ERRNO_UNINIT => "Citadel client is not yet initialized",
            // TODO: Refactor error type system into enum with descriptions
            _ => "Other error",
        };
        self.set_error_details(err_no, message);
    }

    #[node_bindgen]
    pub(crate) fn set_error(&mut self, err: citadel::Error) {
        let err_no = match err {
            Error::Io(_) => ERRNO_IO,
            Error::Rpc(_) => ERRNO_RPC,
            Error::Networking(_) => ERRNO_NET,
            Error::Transport(_) => ERRNO_TRANSPORT,
            Error::NotSupported(_) => ERRNO_NOTSUPPORTED,
            Error::StorageDriver(_) => ERRNO_STORAGE,
            Error::ServerFailure(_) => ERRNO_SERVERFAIL,
            Error::EmbeddedNodeInitError => ERRNO_EMBEDDEDFAIL,
            _ => c_int::MAX,
        };
        self.set_error_details(err_no, &err.to_string());
    }

    #[node_bindgen]
    pub(crate) fn set_failure(&mut self, failure: microservices::rpc::Failure) {
        self.set_error_details(ERRNO_SERVERFAIL, failure);
    }

    #[node_bindgen]
    pub(crate) fn is_ok(&self) -> bool {
        self.message.is_null() && self.err_no == SUCCESS
    }

    #[node_bindgen]
    pub(crate) fn has_err(&self) -> bool {
        self.err_no != SUCCESS && !self.message.is_null()
    }

    #[node_bindgen]
    pub(crate) fn process_response(
        &mut self,
        response: Result<rpc::Reply, Error>,
    ) -> *const c_char {
        response
            .map_err(|err| {
                self.set_error(err);
                ()
            })
            .and_then(|reply| {
                if let rpc::Reply::Failure(failure) = reply {
                    self.set_failure(failure);
                    Err(())
                } else {
                    Ok(reply)
                }
            })
            .map(|result| match result.inner_to_json() {
                Ok(json) => {
                    self.set_success();
                    json.try_into_raw().unwrap_or(ptr::null())
                }
                Err(err) => {
                    self.set_error_details(
                        ERRNO_JSON,
                        &format!("Unable to JSON-encode response: {}", err),
                    );
                    ptr::null()
                }
            })
            .unwrap_or(ptr::null())
    }

    #[node_bindgen]
    pub(crate) fn parse_string<'a>(
        &mut self,
        s: *const c_char,
        arg_name: &'a str,
    ) -> Result<&'a str, ()> {
        match s.try_as_str() {
            Some(s) => Ok(s),
            None => Err(self.set_error_details(
                ERRNO_NULL,
                &format!("{} can't be null", arg_name),
            )),
        }
    }

    #[node_bindgen]
    pub(crate) fn parse_contract_id(
        &mut self,
        bech32: *const c_char,
    ) -> Result<citadel::model::ContractId, ()> {
        match bech32.try_as_str() {
            Some(s) => citadel::model::ContractId::from_str(s).map_err(|err| {
                self.set_error_details(
                    ERRNO_PARSE,
                    &format!("invalid wallet contract id: {}", err),
                )
            }),
            None => Err(self.set_error_details(
                ERRNO_NULL,
                "null value instead of valid wallet contract id",
            )),
        }
    }

    #[node_bindgen]
    pub(crate) fn parse_asset_id(
        &mut self,
        bech32: *const c_char,
    ) -> Result<Option<rgb::ContractId>, ()> {
        bech32
            .try_as_str()
            .map(rgb::ContractId::from_str)
            .transpose()
            .map_err(|err| {
                self.set_error_details(
                    ERRNO_PARSE,
                    &format!("invalid RGB asset id: {}", err),
                )
            })
    }
}
