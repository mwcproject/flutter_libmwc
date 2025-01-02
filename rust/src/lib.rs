use std::cmp::Ordering;
use std::os::raw::c_char;
use std::ffi::{CString, CStr, c_void};
use std::path::Path;
use std::sync::Arc;
use rand::thread_rng;
use rustc_serialize::json::decode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mwc_wallet_api::{self, Owner};
use mwc_wallet_config::{WalletConfig, MQSConfig};
use mwc_wallet_libwallet::api_impl::types::{InitTxArgs, InitTxSendArgs};
use mwc_wallet_libwallet::api_impl::owner;
use mwc_wallet_impls::{DefaultLCProvider, DefaultWalletImpl, MWCMQSAddress, Address, AddressType, HTTPNodeClient};
use mwc_keychain::mnemonic;
use mwc_wallet_util::mwc_core::global::ChainTypes;
use mwc_wallet_util::mwc_core::global;
use mwc_util::file::get_first_line;
use mwc_wallet_util::mwc_util::ZeroingString;
use mwc_util::Mutex;
use mwc_wallet_libwallet::{scan, Slate, SlatePurpose, SlateVersion, VersionedSlate, wallet_lock, NodeClient, WalletInst, WalletLCProvider, Error, proof::proofaddress as proofaddress};
use mwc_wallet_controller::{controller, Error as MWCWalletControllerError};
use mwc_wallet_libwallet::proof::proofaddress::ProvableAddress;
use mwc_wallet_util::mwc_keychain::{Keychain, ExtKeychain};

use mwc_util::secp::rand::Rng;
use mwc_util::init_logger as mwc_wallet_init_logger;
use mwc_util::logger::LoggingConfig;
use mwc_util::secp::key::{self, SecretKey};
use ed25519_dalek::{PublicKey as DalekPublicKey, SecretKey as DalekSecretKey};
use android_logger::FilterBuilder;
use mwc_wallet_impls::Subscriber;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub wallet_dir: String,
    pub check_node_api_http_addr: String,
    pub chain: String,
    pub account: Option<String>
}


type Wallet = Arc<
    Mutex<
        Box<
            dyn WalletInst<
                'static,
                DefaultLCProvider<'static, HTTPNodeClient, ExtKeychain>,
                HTTPNodeClient,
                ExtKeychain,
            >,
        >,
    >,
>;

macro_rules! ensure_wallet (
    ($wallet_ptr:expr, $wallet:ident) => (
        if ($wallet_ptr as *mut Wallet).as_mut().is_none() {
            println!("{}", "WALLET_IS_NOT_OPEN");
        }
        let $wallet = ($wallet_ptr as *mut Wallet).as_mut().unwrap();
    )
);


fn init_logger() {
    android_logger::init_once(
        AndroidConfig::default()
            .with_min_level(Level::Trace)
            .with_tag("libmwc")
            .with_filter(FilterBuilder::new().parse("debug,mwc-wallet::crate=super").build()),
    );
}

impl Config {
    fn from_str(json: &str) -> Result<Self, serde_json::error::Error> {
        let result = match  serde_json::from_str::<Config>(json) {
            Ok(config) => {
                config
            }, Err(err) => {
                return  Err(err);
            }
        };
        Ok(result)
    }
}

/*
    Create Wallet config
*/
fn create_wallet_config(config: Config) -> Result<WalletConfig, Error> {
    let chain_type = match config.chain.as_ref() {
        "mainnet" => global::ChainTypes::Mainnet,
        "floonet" => global::ChainTypes::Floonet,
        "usertesting" => global::ChainTypes::UserTesting,
        "automatedtesting" => global::ChainTypes::AutomatedTesting,
        _ => global::ChainTypes::Floonet,
    };

    let api_secret_path = config.wallet_dir.clone() + "/.api_secret";
    Ok(WalletConfig {
        data_file_dir: config.wallet_dir,
        chain_type: Some(chain_type),
        api_listen_port: 3415,
        api_secret_path: None,
        node_api_secret_path: if Path::new(&api_secret_path).exists() {
            Some(api_secret_path)
        } else {
            None
        },
        check_node_api_http_addr: config.check_node_api_http_addr,
        ..WalletConfig::default()
    })
}

#[macro_use] extern crate log;
extern crate android_logger;
extern crate simplelog;

use log::Level;
use android_logger::Config as AndroidConfig;
use ffi_helpers::{export_task, Task};
use ffi_helpers::task::{CancellationToken, TaskHandle};

/*
    Create a new wallet
*/

#[no_mangle]
pub unsafe extern "C" fn wallet_init(
    config: *const c_char,
    mnemonic: *const c_char,
    password: *const c_char,
    name: *const c_char
) -> *const c_char {

    let result = match _wallet_init(config, mnemonic, password, name) {
        Ok(created) => {
            created
        }, Err(e ) => {
            let error_msg = format!("Error {}", &e.to_string());
            let error_msg_ptr = CString::new(error_msg).unwrap();
            let ptr = error_msg_ptr.as_ptr(); // Get a pointer to the underlaying memory for s
            std::mem::forget(error_msg_ptr);
            ptr
        }
    };
    result
}

#[no_mangle]
pub unsafe extern "C" fn get_mnemonic() -> *const c_char {
    let result = match _get_mnemonic() {
        Ok(phrase) => {
            phrase
        }, Err(e ) => {
            let error_msg = format!("Error {}", &e.to_string());
            let error_msg_ptr = CString::new(error_msg).unwrap();
            let ptr = error_msg_ptr.as_ptr(); // Get a pointer to the underlaying memory for s
            std::mem::forget(error_msg_ptr);
            ptr
        }
    };
    result
}


fn _get_mnemonic() -> Result<*const c_char, mnemonic::Error> {
    let mut wallet_phrase = "".to_string();
    match mnemonic() {
        Ok(phrase) => {
            wallet_phrase.push_str(&phrase);
        },Err(e) => {
            return Err(e);
        }
    }
    let s = CString::new(wallet_phrase).unwrap();
    let p = s.as_ptr(); // Get a pointer to the underlaying memory for s
    std::mem::forget(s); // Give up the responsibility of cleaning up/freeing s
    Ok(p)
}

fn _wallet_init(
    config: *const c_char,
    mnemonic: *const c_char,
    password: *const c_char,
    name: *const c_char
) -> Result<*const c_char, Error> {

    let config = unsafe { CStr::from_ptr(config) };
    let mnemonic = unsafe { CStr::from_ptr(mnemonic) };
    let password = unsafe { CStr::from_ptr(password) };
    let name = unsafe { CStr::from_ptr(name) };

    let str_password = match password.to_str() {
        Ok(str_pass) => {str_pass}, Err(e) => {return Err(
            Error::GenericError(format!("{}", e.to_string()))
        )}
    };

    let str_config = match config.to_str() {
        Ok(str_conf) => {str_conf}, Err(e) => {return Err(
            Error::GenericError(format!("{}", e.to_string()))
        )}
    };

    let phrase = match mnemonic.to_str() {
        Ok(str_phrase) => {str_phrase}, Err(e) => {return Err(
            Error::GenericError(format!("{}", e.to_string()))
        )}
    };

    let str_name = match name.to_str() {
        Ok(str_name) => {str_name}, Err(e) => {return Err(
            Error::GenericError(format!("{}", e.to_string()))
        )}
    };

    let mut create_msg = "".to_string();
    match create_wallet(str_config, phrase, str_password, str_name) {
        Ok(_) => {
            create_msg.push_str("");
        },Err(e) => {
            return Err(e);
        }
    }
    let s = CString::new(create_msg).unwrap();
    let p = s.as_ptr(); // Get a pointer to the underlaying memory for s
    std::mem::forget(s); // Give up the responsibility of cleaning up/freeing s
    Ok(p)
}


#[no_mangle]
pub unsafe extern "C"  fn rust_open_wallet(
    config: *const c_char,
    password: *const c_char,
) -> *const c_char {
    //init_logger();
    let result = match _open_wallet(
        config,
        password
    ) {
        Ok(wallet) => {
            wallet
        }, Err(e ) => {
            let error_msg = format!("Error {}", &e.to_string());
            let error_msg_ptr = CString::new(error_msg).unwrap();
            let ptr = error_msg_ptr.as_ptr(); // Get a pointer to the underlaying memory for s
            std::mem::forget(error_msg_ptr);
            ptr
        }
    };
    result
}

fn _open_wallet(
    config: *const c_char,
    password: *const c_char,
) -> Result<*const c_char, Error> {
    let c_conf = unsafe { CStr::from_ptr(config) };
    let c_password = unsafe { CStr::from_ptr(password) };

    let str_config = c_conf.to_str().unwrap();
    let str_password = c_password.to_str().unwrap();

    let mut result = String::from("");
    match open_wallet(&str_config, str_password) {
        Ok(res) => {
            let wlt = res.0;
            let sek_key = res.1;
            let wallet_int = Box::into_raw(Box::new(wlt)) as i64;
            let wallet_data = (wallet_int, sek_key);
            let wallet_ptr = serde_json::to_string(&wallet_data).unwrap();
            result.push_str(&wallet_ptr);
        }
        Err(err) => {
            return Err(err);
        }
    };

    let s = CString::new(result).unwrap();
    let p = s.as_ptr(); // Get a pointer to the underlaying memory for s
    std::mem::forget(s); // Give up the responsibility of cleaning up/freeing s
    Ok(p)
}


/*
    Get wallet info
    This contains wallet balances
*/
#[no_mangle]
pub unsafe extern "C"  fn rust_wallet_balances(
    wallet: *const c_char,
    refresh: *const c_char,
    min_confirmations: *const c_char,
) -> *const c_char {
    let wallet_ptr = CStr::from_ptr(wallet);
    let c_refresh = CStr::from_ptr(refresh);
    let minimum_confirmations = CStr::from_ptr(min_confirmations);
    let minimum_confirmations: u64 = minimum_confirmations.to_str().unwrap().to_string().parse().unwrap();

    let refresh_from_node: u64 = c_refresh.to_str().unwrap().to_string().parse().unwrap();
    let refresh = match refresh_from_node {
        0 => false,
        _=> true
    };

    let wallet_data = wallet_ptr.to_str().unwrap();
    let tuple_wallet_data: (i64, Option<SecretKey>) = serde_json::from_str(wallet_data).unwrap();
    let wlt = tuple_wallet_data.0;
    let sek_key = tuple_wallet_data.1;

    ensure_wallet!(wlt, wallet);

    let result = match _wallet_balances(
        wallet,
        sek_key,
        refresh,
        minimum_confirmations
    ) {
        Ok(balances) => {
            balances
        }, Err(e ) => {
            let error_msg = format!("Error {}", &e.to_string());
            let error_msg_ptr = CString::new(error_msg).unwrap();
            let ptr = error_msg_ptr.as_ptr(); // Get a pointer to the underlaying memory for s
            std::mem::forget(error_msg_ptr);
            ptr
        }
    };
    result
}

fn _wallet_balances(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    refresh: bool,
    min_confirmations: u64,
) -> Result<*const c_char, Error> {
    let mut wallet_info = "".to_string();
    match get_wallet_info(
        &wallet,
        keychain_mask,
        refresh,
        min_confirmations
    ) {
        Ok(info) => {
            let str_wallet_info = serde_json::to_string(&info).unwrap();
            wallet_info.push_str(&str_wallet_info);
        },Err(e) => {
            return Err(e);
        }
    }
    let s = CString::new(wallet_info).unwrap();
    let p = s.as_ptr(); // Get a pointer to the underlaying memory for s
    std::mem::forget(s); // Give up the responsibility of cleaning up/freeing s
    Ok(p)
}



#[no_mangle]
pub unsafe extern "C"  fn rust_recover_from_mnemonic(
    config: *const c_char,
    password: *const c_char,
    mnemonic: *const c_char,
    name: *const c_char
) -> *const c_char {

    let result = match _recover_from_mnemonic(
        config,
        password,
        mnemonic,
        name
    ) {
        Ok(recovered) => {
            recovered
        }, Err(e ) => {
            let error_msg = format!("Error {}", &e.to_string());
            let error_msg_ptr = CString::new(error_msg).unwrap();
            let ptr = error_msg_ptr.as_ptr(); // Get a pointer to the underlaying memory for s
            std::mem::forget(error_msg_ptr);
            ptr
        }
    };
    result
}

fn _recover_from_mnemonic(
    config: *const c_char,
    password: *const c_char,
    mnemonic: *const c_char,
    name: *const c_char
) -> Result<*const c_char, Error> {
    let c_conf = unsafe { CStr::from_ptr(config) };
    let c_password = unsafe { CStr::from_ptr(password) };
    let c_mnemonic = unsafe { CStr::from_ptr(mnemonic) };
    let c_name = unsafe { CStr::from_ptr(name) };

    let input_conf = c_conf.to_str().unwrap();
    let str_password = c_password.to_str().unwrap();
    let wallet_config = match Config::from_str(&input_conf.to_string()) {
        Ok(config) => {
            config
        }, Err(err) => {
            return Err(Error::GenericError(format!(
                "Wallet config error : {}",
                err.to_string()
            )))
        }
    };
    let phrase = c_mnemonic.to_str().unwrap();
    let name = c_name.to_str().unwrap();

    let mut recover_response = "".to_string();
    match recover_from_mnemonic(phrase, str_password, &wallet_config, name) {
        Ok(_)=> {
            recover_response.push_str("recovered");
        },
        Err(e)=> {
            return Err(e);
        }
    }
    let s = CString::new(recover_response).unwrap();
    let p = s.as_ptr(); // Get a pointer to the underlaying memory for s
    std::mem::forget(s); // Give up the responsibility of cleaning up/freeing s
    Ok(p)
}

#[no_mangle]
pub unsafe extern "C" fn rust_wallet_scan_outputs(
    wallet: *const c_char,
    start_height: *const c_char,
    number_of_blocks: *const c_char,
) -> *const c_char {
    let wallet_ptr = CStr::from_ptr(wallet);
    let c_start_height = CStr::from_ptr(start_height);
    let c_number_of_blocks = CStr::from_ptr(number_of_blocks);
    let start_height: u64 = c_start_height.to_str().unwrap().to_string().parse().unwrap();
    let number_of_blocks: u64 = c_number_of_blocks.to_str().unwrap().to_string().parse().unwrap();

    let wallet_data = wallet_ptr.to_str().unwrap();
    let tuple_wallet_data: (i64, Option<SecretKey>) = serde_json::from_str(wallet_data).unwrap();
    let wlt = tuple_wallet_data.0;
    let sek_key = tuple_wallet_data.1;

    ensure_wallet!(wlt, wallet);

    let result = match _wallet_scan_outputs(
        wallet,
        sek_key,
        start_height,
        number_of_blocks
    ) {
        Ok(scan) => {
            scan
        }, Err(e ) => {
            let error_msg = format!("Error {}", &e.to_string());
            let error_msg_ptr = CString::new(error_msg).unwrap();
            let ptr = error_msg_ptr.as_ptr(); // Get a pointer to the underlaying memory for s
            std::mem::forget(error_msg_ptr);
            ptr
        }
    };
    result
}

fn _wallet_scan_outputs(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    start_height: u64,
    number_of_blocks: u64
) -> Result<*const c_char, Error> {
    let mut scan_result = String::from("");
    match wallet_scan_outputs(
        &wallet,
        keychain_mask,
        Some(start_height),
        Some(number_of_blocks)
    ) {
        Ok(scan) => {
            scan_result.push_str(&scan);
        },
        Err(err) => {
            return Err(err);
        },
    }

    let s = CString::new(scan_result).unwrap();
    let p = s.as_ptr(); // Get a pointer to the underlaying memory for s
    std::mem::forget(s); // Give up the responsibility of cleaning up/freeing s
    Ok(p)
}

#[no_mangle]
pub unsafe extern "C" fn rust_create_tx(
    wallet: *const c_char,
    amount: *const c_char,
    to_address: *const c_char,
    secret_key_index: *const c_char,
    mwcmqs_config: *const c_char,
    confirmations: *const c_char,
    note: *const c_char
) -> *const c_char {

    let wallet_data = CStr::from_ptr(wallet).to_str().unwrap();
    let min_confirmations: u64 = CStr::from_ptr(confirmations).to_str().unwrap().to_string().parse().unwrap();
    let amount: u64 = CStr::from_ptr(amount).to_str().unwrap().to_string().parse().unwrap();
    let address = CStr::from_ptr(to_address).to_str().unwrap();
    let note = CStr::from_ptr(note).to_str().unwrap();
    let key_index: u32 = CStr::from_ptr(secret_key_index).to_str().unwrap().parse().unwrap();
    let mwcmqs_config = CStr::from_ptr(mwcmqs_config).to_str().unwrap();

    let tuple_wallet_data: (i64, Option<SecretKey>) = serde_json::from_str(wallet_data).unwrap();

    let _listen = Listener {
        wallet_ptr_str: wallet_data.to_string(),
        mwcmqs_config: mwcmqs_config.parse().unwrap()
    };

    let wlt = tuple_wallet_data.0;
    let sek_key = tuple_wallet_data.1;
    ensure_wallet!(wlt, wallet);
    let result = match _create_tx(
        wallet,
        sek_key,
        amount,
        address,
        key_index,
        mwcmqs_config,
        min_confirmations,
        note
    ) {
        Ok(slate) => {
            //Spawn listener again
            //listener_spawn(&listen);
            slate
        }, Err(e ) => {
            let error_msg = format!("Error {}", &e.to_string());
            let error_msg_ptr = CString::new(error_msg).unwrap();
            let ptr = error_msg_ptr.as_ptr(); // Get a pointer to the underlaying memory for s
            std::mem::forget(error_msg_ptr);
            ptr
        }
    };
    result

}

fn _create_tx(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    amount: u64,
    address: &str,
    _secret_key_index: u32,
    mwcmqs_config: &str,
    minimum_confirmations: u64,
    note: &str
) -> Result<*const c_char, Error> {
    let  mut message = String::from("");
    match tx_create(
        &wallet,
        keychain_mask.clone(),
        amount,
        minimum_confirmations,
        false,
        mwcmqs_config,
        address,
        note) {
        Ok(slate) => {
            let empty_json = format!(r#"{{"slate_msg": ""}}"#);
            let create_response = (&slate, &empty_json);
            let str_create_response = serde_json::to_string(&create_response).unwrap();
            message.push_str(&str_create_response);
        },
        Err(e) => {
            message.push_str(&e.to_string());
            return Err(e);
        }
    }

    let s = CString::new(message).unwrap();
    let p = s.as_ptr(); // Get a pointer to the underlaying memory for s
    std::mem::forget(s); // Give up the responsibility of cleaning up/freeing s
    Ok(p)


}

#[no_mangle]
pub unsafe extern "C" fn rust_txs_get(
    wallet: *const c_char,
    refresh_from_node: *const c_char,
) -> *const c_char {
    let c_wallet = CStr::from_ptr(wallet);
    let c_refresh_from_node = CStr::from_ptr(refresh_from_node);
    let refresh_from_node: u64 = c_refresh_from_node.to_str().unwrap().to_string().parse().unwrap();
    let refresh = match refresh_from_node {
        0 => false,
        _=> true
    };

    let wallet_data = c_wallet.to_str().unwrap();
    let tuple_wallet_data: (i64, Option<SecretKey>) = serde_json::from_str(wallet_data).unwrap();
    let wlt = tuple_wallet_data.0;
    let sek_key = tuple_wallet_data.1;

    ensure_wallet!(wlt, wallet);

    let result = match _txs_get(
        wallet,
        sek_key,
        refresh,
    ) {
        Ok(txs) => {
            txs
        }, Err(e ) => {
            let error_msg = format!("Error {}", &e.to_string());
            let error_msg_ptr = CString::new(error_msg).unwrap();
            let ptr = error_msg_ptr.as_ptr(); // Get a pointer to the underlaying memory for s
            std::mem::forget(error_msg_ptr);
            ptr
        }
    };
    result
}

fn _txs_get(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    refresh_from_node: bool,
) -> Result<*const c_char, Error> {
    let mut txs_result = "".to_string();
    match txs_get(
        wallet,
        keychain_mask,
        refresh_from_node
    ) {
        Ok(txs) => {
            txs_result.push_str(&txs);
        },
        Err(err) => {
            return Err(err);
        },
    }

    let s = CString::new(txs_result).unwrap();
    let p = s.as_ptr(); // Get a pointer to the underlaying memory for s
    std::mem::forget(s); // Give up the responsibility of cleaning up/freeing s
    Ok(p)
}

#[no_mangle]
pub unsafe extern "C" fn rust_tx_cancel(
    wallet: *const c_char,
    tx_id: *const c_char,
) -> *const c_char {

    let wallet_ptr = CStr::from_ptr(wallet);
    let tx_id = CStr::from_ptr(tx_id);
    let tx_id = tx_id.to_str().unwrap();
    let uuid = Uuid::parse_str(tx_id).map_err(|e| MWCWalletControllerError::GenericError(e.to_string())).unwrap();

    let wallet_data = wallet_ptr.to_str().unwrap();
    let tuple_wallet_data: (i64, Option<SecretKey>) = serde_json::from_str(wallet_data).unwrap();
    let wlt = tuple_wallet_data.0;
    let sek_key = tuple_wallet_data.1;

    ensure_wallet!(wlt, wallet);

    let result = match _tx_cancel(
        wallet,
        sek_key,
        uuid,
    ) {
        Ok(cancelled) => {
            cancelled
        }, Err(e ) => {
            let error_msg = format!("Error {}", &e.to_string());
            let error_msg_ptr = CString::new(error_msg).unwrap();
            let ptr = error_msg_ptr.as_ptr(); // Get a pointer to the underlaying memory for s
            std::mem::forget(error_msg_ptr);
            ptr
        }
    };
    result
}

fn _tx_cancel(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    tx_id: Uuid,
) -> Result<*const c_char, Error>{
    let mut cancel_msg = "".to_string();
    match  tx_cancel(wallet, keychain_mask, tx_id) {
        Ok(_) => {
            cancel_msg.push_str("");
        },Err(err) => {
            return Err(err);
        }
    }
    let s = CString::new(cancel_msg).unwrap();
    let p = s.as_ptr(); // Get a pointer to the underlaying memory for s
    std::mem::forget(s); // Give up the responsibility of cleaning up/freeing s
    Ok(p)
}

#[no_mangle]
pub unsafe extern "C" fn rust_get_chain_height(
    config: *const c_char,
) -> *const c_char {
    let result = match _get_chain_height(
        config
    ) {
        Ok(chain_height) => {
            chain_height
        }, Err(e ) => {
            let error_msg = format!("Error {}", &e.to_string());
            let error_msg_ptr = CString::new(error_msg).unwrap();
            let ptr = error_msg_ptr.as_ptr(); // Get a pointer to the underlaying memory for s
            std::mem::forget(error_msg_ptr);
            ptr
        }
    };
    result
}

fn _get_chain_height(config: *const c_char) -> Result<*const c_char, Error> {
    let c_config = unsafe { CStr::from_ptr(config) };
    let str_config = c_config.to_str().unwrap();
    let mut chain_height = "".to_string();
    match get_chain_height(&str_config) {
        Ok(chain_tip) => {
            chain_height.push_str(&chain_tip.to_string());
        },
        Err(e) => {
            debug!("CHAIN_HEIGHT_ERROR {}", e.to_string());
            return Err(e);
        },
    }
    let s = CString::new(chain_height).unwrap();
    let p = s.as_ptr(); // Get a pointer to the underlaying memory for s
    std::mem::forget(s); // Give up the responsibility of cleaning up/freeing s
    Ok(p)
}

pub fn _init_logs(config: &str) -> Result<*const c_char, Error> {
    let config = match Config::from_str(&config.to_string()) {
        Ok(config) => config,
        Err(_e) => {
            return Err(Error::GenericError(format!(
                "{}",
                "Unable to get mwc wallet config"
            )))
        }
    };

    let log = LoggingConfig {
        file_log_level: Level::Trace,
        log_file_path: format!("{}/mwc-wallet.log", config.wallet_dir),
        ..LoggingConfig::default()
    };
    let _ = mwc_wallet_init_logger(Some(log), None);
    let success_msg = CString::new("Logger initialized successfully").unwrap();
    Ok(success_msg.into_raw())
}

#[no_mangle]
pub unsafe extern "C" fn rust_init_logs(config: *const c_char) -> *const c_char {
    // Convert C string to Rust string
    let c_conf = CStr::from_ptr(config);
    let wallet_dir_str = match c_conf.to_str() {
        Ok(s) => s,
        Err(_) => {
            let err_msg = CString::new("Failed to parse config string").unwrap();
            return err_msg.into_raw();
        }
    };
    let _ =_init_logs(wallet_dir_str);
    println!("Logger initialized successfully");
    let success_msg = CString::new("Logger initialized successfully").unwrap();
    success_msg.into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn rust_delete_wallet(
    _wallet: *const c_char,
    config: *const c_char,
) -> *const c_char  {
    let c_conf = CStr::from_ptr(config);
    let _config = Config::from_str(c_conf.to_str().unwrap()).unwrap(); // TODO handle error here

    let result = match _delete_wallet(
        _config,
    ) {
        Ok(deleted) => {
            deleted
        }, Err(err) => {
            let error_msg = format!("Error deleting wallet from _delete_wallet in rust_delete_wallet {}", &err.to_string());
            let error_msg_ptr = CString::new(error_msg).unwrap();
            let ptr = error_msg_ptr.as_ptr(); // Get a pointer to the underlaying memory for s
            std::mem::forget(error_msg_ptr);
            ptr
        }
    };
    result
}

fn _delete_wallet(
    config: Config,
) -> Result<*const c_char, Error> {
    let mut delete_result = String::from("");
    match delete_wallet(config) {
        Ok(deleted) => {
            delete_result.push_str(&deleted);
        },
        Err(err) => {
            return Err(err);
        },
    }
    let s = CString::new(delete_result).unwrap();
    let p = s.as_ptr(); // Get a pointer to the underlaying memory for s
    std::mem::forget(s); // Give up the responsibility of cleaning up/freeing s
    Ok(p)

}
#[no_mangle]
pub unsafe extern "C" fn rust_preview_slatepack(
    wallet: *const c_char,
    message: *const c_char,
) -> *const c_char {
    let c_wallet = CStr::from_ptr(wallet);
    let wallet_data = c_wallet.to_str().unwrap();
    let tuple_wallet_data: (i64, Option<SecretKey>) = serde_json::from_str(wallet_data).unwrap();
    let wlt = tuple_wallet_data.0;
    let sek_key = tuple_wallet_data.1;
    ensure_wallet!(wlt, wallet);

    let c_message = CStr::from_ptr(message);
    let message_str = c_message.to_str().unwrap();

    match preview_slatepack(wallet, sek_key, message_str.to_string()) {
        Ok(json_result) => {
            let c_string = CString::new(json_result).unwrap();
            let ptr = c_string.as_ptr();
            std::mem::forget(c_string);
            ptr
        }
        Err(err) => {
            let error_msg = format!("Error: {}", err);
            let error_msg_ptr = CString::new(error_msg).unwrap();
            let ptr = error_msg_ptr.as_ptr();
            std::mem::forget(error_msg_ptr);
            ptr
        }
    }
}

fn preview_slatepack(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    message: String,
) -> Result<String, Error> {
    let api = Owner::new(wallet.clone(), None, None);

    let decoded_slatepack = api.decrypt_slatepack(
        keychain_mask.as_ref(),
        VersionedSlate::SP(message),
        Some(0),
    );

    match decoded_slatepack {
        Ok((slate, purpose, sender_pub_key, recipient_pub_key)) => {
            // Serialize the tuple into a JSON object
            let preview_data = serde_json::to_string(&serde_json::json!({
                "slate": slate,
                "purpose": purpose,
                "sender_pub_key": sender_pub_key.map(|pk| ProvableAddress::from_tor_pub_key(&pk)),
                "recipient_pub_key": recipient_pub_key.map(|pk| ProvableAddress::from_tor_pub_key(&pk))
            }))
            .map_err(|e| Error::GenericError(e.to_string()))?;
            Ok(preview_data)
        }
        Err(err) => Err(err),
    }
}


#[no_mangle]
pub unsafe extern "C" fn rust_tx_send_slatepack(
    wallet: *const c_char,
    selection_strategy_is_use_all: *const c_char,
    minimum_confirmations: *const c_char,
    message: *const c_char,
    amount: *const c_char,
    address: *const c_char,
) -> *const c_char  {
    let c_wallet = CStr::from_ptr(wallet);
    let c_strategy_is_use_all = CStr::from_ptr(selection_strategy_is_use_all);
    let strategy_is_use_all: u64 = c_strategy_is_use_all.to_str().unwrap().to_string().parse().unwrap();
    let strategy_use_all = match strategy_is_use_all {
        0 => false,
        _=> true
    };
    let c_minimum_confirmations = CStr::from_ptr(minimum_confirmations);
    let minimum_confirmations: u64 = c_minimum_confirmations.to_str().unwrap().to_string().parse().unwrap();
    let c_message = CStr::from_ptr(message);
    let str_message = c_message.to_str().unwrap();
    let c_amount = CStr::from_ptr(amount);
    let amount: u64 = c_amount.to_str().unwrap().to_string().parse().unwrap();
    let c_address = CStr::from_ptr(address);
    let str_address = c_address.to_str().unwrap();

    let wallet_data = c_wallet.to_str().unwrap();
    let tuple_wallet_data: (i64, Option<SecretKey>) = serde_json::from_str(wallet_data).unwrap();
    let wlt = tuple_wallet_data.0;
    let sek_key = tuple_wallet_data.1;
    ensure_wallet!(wlt, wallet);

    let result = match _tx_send_slatepack(
        wallet,
        sek_key,
        strategy_use_all,
        minimum_confirmations,
        str_message,
        amount,
        str_address
    ) {
        Ok(tx_data) => {
            tx_data
        }, Err(err ) => {
            let error_msg = format!("Error {}", &err.to_string());
            let error_msg_ptr = CString::new(error_msg).unwrap();
            let ptr = error_msg_ptr.as_ptr(); // Get a pointer to the underlaying memory for s
            std::mem::forget(error_msg_ptr);
            ptr
        }
    };
    result
}

fn _tx_send_slatepack(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    selection_strategy_is_use_all: bool,
    minimum_confirmations: u64,
    message: &str,
    amount: u64,
    address: &str
) -> Result<*const c_char, Error> {
    let mut send_result = String::from("");
    match tx_send_slatepack(
        wallet,
        keychain_mask,
        selection_strategy_is_use_all,
        minimum_confirmations,
        message,
        amount,
        address
    ) {
        Ok(sent) => {
            let empty_json = format!(r#"{{"slate_msg": ""}}"#);
            let create_response = (&sent, &empty_json);
            let str_create_response = serde_json::to_string(&create_response).unwrap();
            send_result.push_str(&str_create_response);
        },
        Err(err) => {
            return Err(err);
        },
    }
    let s = CString::new(send_result).unwrap();
    let p = s.as_ptr(); // Get a pointer to the underlaying memory for s
    std::mem::forget(s); // Give up the responsibility of cleaning up/freeing s
    Ok(p)
}


pub fn tx_send_slatepack(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    selection_strategy_is_use_all: bool,
    minimum_confirmations: u64,
    message: &str,
    amount: u64,
    address: &str
) -> Result<String, Error> {
    let api = Owner::new(wallet.clone(), None, None);

    // Handle optional message
    let message = if message.is_empty() { None } else { Some(message.to_string()) };

    // Parse recipient address
    let slatepack_recipient: Option<ProvableAddress> = if address.is_empty() {
        None
    } else {
        Some(ProvableAddress::from_str(address).map_err(|e| {
            Error::GenericError(format!("Unable to parse slatepack_recipient, {}", e))
        })?)
    };

    // Prepare transaction arguments
    let args = InitTxArgs {
        src_acct_name: Some("default".to_string()),
        amount,
        minimum_confirmations,
        max_outputs: 500,
        num_change_outputs: 1,
        selection_strategy_is_use_all,
        message,
        slatepack_recipient: slatepack_recipient.clone(),
        target_slate_version: Some(4),  // Wrap in Some()
        ..Default::default()
    };

    // Initialize transaction
    match api.init_send_tx(keychain_mask.as_ref(), &args, 1) {
        Ok(slate) => {
            println!("{}", "CREATE_TX_SLATEPACK_SUCCESS");

            // Handle recipient tor address if available
            let recipient = slatepack_recipient.as_ref().map(|recipient| {
                recipient.tor_public_key().map_err(|e| {
                    Error::SlatepackEncodeError(format!("Expecting recipient tor address, {}", e))
                })
            }).transpose()?;

            // Encrypt the slate
            let encoded_slatepack = api.encrypt_slate(
                keychain_mask.as_ref(),
                &slate,
                Some(SlateVersion::SP),
                SlatePurpose::SendInitial,
                recipient.clone(),
                None,
                false,
            );

            // Handle encoded slatepack result
            match encoded_slatepack {
                Ok(vs) => {
                    // Lock outputs
                    api.tx_lock_outputs(keychain_mask.as_ref(), &slate, None, 0).map_err(|e| {
                        Error::SlatepackEncodeError(format!("Failed to lock outputs: {}", e))
                    })?;

                    // Retrieve transaction details
                    let txs = api.retrieve_txs(keychain_mask.as_ref(), false, None, Some(slate.id), None)
                        .map_err(|e| e)?;

                    // Prepare transaction data
                    let tx_data = (
                        serde_json::to_string(&txs.1).unwrap(),
                        serde_json::to_string(&vs).unwrap()
                    );

                    let str_tx_data = serde_json::to_string(&tx_data).unwrap();
                    Ok(str_tx_data)
                }
                Err(e) => Err(e),
            }
        }
        Err(err) => {
            println!("CREATE_TX_ERROR_IN_SLATEPACK_SEND {}", err.to_string());
            Err(err)
        }
    }
}


#[no_mangle]
pub unsafe extern "C" fn rust_tx_send_http(
    wallet: *const c_char,
    selection_strategy_is_use_all: *const c_char,
    minimum_confirmations: *const c_char,
    message: *const c_char,
    amount: *const c_char,
    address: *const c_char,
) -> *const c_char  {
    let c_wallet = CStr::from_ptr(wallet);
    let c_strategy_is_use_all = CStr::from_ptr(selection_strategy_is_use_all);
    let strategy_is_use_all: u64 = c_strategy_is_use_all.to_str().unwrap().to_string().parse().unwrap();
    let strategy_use_all = match strategy_is_use_all {
        0 => false,
        _=> true
    };
    let c_minimum_confirmations = CStr::from_ptr(minimum_confirmations);
    let minimum_confirmations: u64 = c_minimum_confirmations.to_str().unwrap().to_string().parse().unwrap();
    let c_message = CStr::from_ptr(message);
    let str_message = c_message.to_str().unwrap();
    let c_amount = CStr::from_ptr(amount);
    let amount: u64 = c_amount.to_str().unwrap().to_string().parse().unwrap();
    let c_address = CStr::from_ptr(address);
    let str_address = c_address.to_str().unwrap();

    let wallet_data = c_wallet.to_str().unwrap();
    let tuple_wallet_data: (i64, Option<SecretKey>) = serde_json::from_str(wallet_data).unwrap();
    let wlt = tuple_wallet_data.0;
    let sek_key = tuple_wallet_data.1;
    ensure_wallet!(wlt, wallet);

    let result = match _tx_send_http(
        wallet,
        sek_key,
        strategy_use_all,
        minimum_confirmations,
        str_message,
        amount,
        str_address
    ) {
        Ok(tx_data) => {
            tx_data
        }, Err(err ) => {
            let error_msg = format!("Error {}", &err.to_string());
            let error_msg_ptr = CString::new(error_msg).unwrap();
            let ptr = error_msg_ptr.as_ptr(); // Get a pointer to the underlaying memory for s
            std::mem::forget(error_msg_ptr);
            ptr
        }
    };
    result
}

fn _tx_send_http(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    selection_strategy_is_use_all: bool,
    minimum_confirmations: u64,
    message: &str,
    amount: u64,
    address: &str
) -> Result<*const c_char, Error> {
    let mut send_result = String::from("");
    match tx_send_http(
        wallet,
        keychain_mask,
        selection_strategy_is_use_all,
        minimum_confirmations,
        message,
        amount,
        address
    ) {
        Ok(sent) => {
            let empty_json = format!(r#"{{"slate_msg": ""}}"#);
            let create_response = (&sent, &empty_json);
            let str_create_response = serde_json::to_string(&create_response).unwrap();
            send_result.push_str(&str_create_response);
        },
        Err(err) => {
            return Err(err);
        },
    }
    let s = CString::new(send_result).unwrap();
    let p = s.as_ptr(); // Get a pointer to the underlaying memory for s
    std::mem::forget(s); // Give up the responsibility of cleaning up/freeing s
    Ok(p)
}

pub fn tx_send_http(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    selection_strategy_is_use_all: bool,
    minimum_confirmations: u64,
    message: &str,
    amount: u64,
    address: &str,
) -> Result<String, Error>{
    let api = Owner::new(wallet.clone(), None, None);
    let message = match message {
        "" => None,
        _ => Some(message.to_string()),
    };
    let init_send_args = InitTxSendArgs {
        method: "http".to_string(),
        dest: address.to_string(),
        finalize: true,
        post_tx: true,
        fluff: true,
        apisecret: None
    };

    let args = InitTxArgs {
        src_acct_name: Some("default".to_string()),
        amount,
        minimum_confirmations,
        max_outputs: 500,
        num_change_outputs: 1,
        selection_strategy_is_use_all,
        message: message,
        send_args: Some(init_send_args),
        ..Default::default()
    };

    match api.init_send_tx(keychain_mask.as_ref(), &args, 1) {
        Ok(slate) => {
            println!("{}", "CREATE_TX_SUCCESS");
            //Get transaction for slate, for UI display
            let txs = match api.retrieve_txs(
                keychain_mask.as_ref(),
                false,
                None,
                Some(slate.id),
                None
            ) {
                Ok(txs_result) => {
                    txs_result
                }, Err(e) => {
                    return Err(e);
                }
            };

            let tx_data = (
                serde_json::to_string(&txs.1).unwrap(),
                serde_json::to_string(&slate).unwrap()
            );
            let str_tx_data = serde_json::to_string(&tx_data).unwrap();
            Ok(str_tx_data)
        } Err(err) => {
            println!("CREATE_TX_ERROR_IN_HTTP_SEND {}", err.to_string());
            return  Err(err);
        }
    }
}


#[no_mangle]
pub unsafe extern "C" fn rust_get_wallet_address(
    wallet: *const c_char,
    index: *const c_char,
) -> *const c_char {
    let wallet_ptr = CStr::from_ptr(wallet);
    let index = CStr::from_ptr(index);
    let index: u32 = index.to_str().unwrap().to_string().parse().unwrap();

    let wallet_data = wallet_ptr.to_str().unwrap();
    let tuple_wallet_data: (i64, Option<SecretKey>) = serde_json::from_str(wallet_data).unwrap();
    let wlt = tuple_wallet_data.0;
    let sek_key = tuple_wallet_data.1;

    ensure_wallet!(wlt, wallet);
    let result = match _get_wallet_address(
        wallet,
        sek_key,
        index,
    ) {
        Ok(address) => {
            address
        }, Err(e ) => {
            let error_msg = format!("Error {}", &e.to_string());
            let error_msg_ptr = CString::new(error_msg).unwrap();
            let ptr = error_msg_ptr.as_ptr(); // Get a pointer to the underlaying memory for s
            std::mem::forget(error_msg_ptr);
            ptr
        }
    };
    result
}

fn _get_wallet_address(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    index: u32,
) -> Result<*const c_char, Error> {
    let address = get_wallet_address(&wallet, keychain_mask, index);
    let s = CString::new(address).unwrap();
    let p = s.as_ptr(); // Get a pointer to the underlaying memory for s
    std::mem::forget(s); // Give up the responsibility of cleaning up/freeing s
    Ok(p)
}

pub fn get_wallet_address(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    _index: u32,
) -> String {
    let api = Owner::new(wallet.clone(), None, None);
    let address = api.get_mqs_address(keychain_mask.as_ref()).unwrap();
    let public_proof_address = proofaddress::ProvableAddress::from_pub_key(&address);
    format!("mwcmqs://{}", public_proof_address.public_key)
}

#[no_mangle]
pub unsafe extern "C" fn rust_validate_address(
    address: *const c_char,
) -> *const c_char {
    let address = unsafe { CStr::from_ptr(address) };
    let str_address = address.to_str().unwrap();
    let validate = validate_address(str_address);
    let return_value = match validate {
        true => 1,
        false => 0
    };

    let s = CString::new(return_value.to_string()).unwrap();
    let p = s.as_ptr(); // Get a pointer to the underlaying memory for s
    std::mem::forget(s); // Give up the responsibility of cleaning up/freeing s
    p
}

#[no_mangle]
pub unsafe extern "C" fn rust_get_tx_fees(
    wallet: *const c_char,
    c_amount: *const c_char,
    min_confirmations: *const c_char,
) -> *const c_char {
    let minimum_confirmations = CStr::from_ptr(min_confirmations);
    let minimum_confirmations: u64 = minimum_confirmations.to_str().unwrap().to_string().parse().unwrap();
    let wallet_ptr = CStr::from_ptr(wallet);

    let amount = CStr::from_ptr(c_amount);
    let amount: u64 = amount.to_str().unwrap().to_string().parse().unwrap();

    let wallet_data = wallet_ptr.to_str().unwrap();
    let tuple_wallet_data: (i64, Option<SecretKey>) = serde_json::from_str(wallet_data).unwrap();
    let wlt = tuple_wallet_data.0;
    let sek_key = tuple_wallet_data.1;

    ensure_wallet!(wlt, wallet);

    let result = match _get_tx_fees(
        &wallet,
        sek_key,
        amount,
        minimum_confirmations,
    ) {
        Ok(fees) => {
            fees
        }, Err(e ) => {
            let error_msg = format!("Error {}", &e.to_string());
            let error_msg_ptr = CString::new(error_msg).unwrap();
            let ptr = error_msg_ptr.as_ptr(); // Get a pointer to the underlaying memory for s
            std::mem::forget(error_msg_ptr);
            ptr
        }
    };
    result
}

fn _get_tx_fees(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    amount: u64,
    minimum_confirmations: u64,
) -> Result<*const c_char, Error> {
    let mut fees_data = "".to_string();
    match tx_strategies(wallet, keychain_mask, amount, minimum_confirmations) {
        Ok(fees) => {
            fees_data.push_str(&fees);
        }, Err(e) => {
            return Err(e);
        }
    }
    let s = CString::new(fees_data).unwrap();
    let p = s.as_ptr(); // Get a pointer to the underlaying memory for s
    std::mem::forget(s); // Give up the responsibility of cleaning up/freeing s
    Ok(p)
}

pub fn create_wallet(config: &str, phrase: &str, password: &str, name: &str) -> Result<String, Error> {
    let wallet_pass = ZeroingString::from(password);
    let wallet_config = match Config::from_str(&config) {
        Ok(config) => {
            config
        }, Err(e) => {
            return  Err(Error::GenericError(format!(
                "Error getting wallet config: {}",
                e.to_string()
            )));
        }
    };

    let wallet = match get_wallet(&wallet_config) {
        Ok(wllet) => {
            wllet
        }
        Err(e) => {
            return  Err(e);
        }
    };
    let mut wallet_lock = wallet.lock();
    let lc = match wallet_lock.lc_provider() {
        Ok(wallet_lc) => {
            wallet_lc
        }
        Err(e) => {
            return  Err(e);
        }
    };
    let rec_phrase = ZeroingString::from(phrase);
    let result = match lc.create_wallet(
        Some(name),
        Some(rec_phrase),
        32,
        wallet_pass,
        false,
        None
    ) {
        Ok(_) => {
            "".to_string()
        },
        Err(e) => {
            e.to_string()
        },
    };
    Ok(result)
}

pub fn get_wallet_secret_key_pair(
    wallet: &Wallet, keychain_mask: Option<SecretKey>, index: u32
) -> Result<(DalekSecretKey, DalekPublicKey), Error>{
    let _parent_key_id = {
        wallet_lock!(wallet, w);
        w.parent_key_id().clone()
    };
    wallet_lock!(wallet, w);

    let k = match w.keychain(keychain_mask.as_ref()) {
        Ok(keychain) => {
            keychain
        }
        Err(err) => {
            return  Err(err);
        }
    };

    let sec_key = match proofaddress::payment_proof_address_dalek_secret(
        &k, Some(index)
    ) {
        Ok(s_key) => {
            s_key
        }
        Err(err) => {
            return Err(err);
        }
    };
    let pub_key = DalekPublicKey::from(&sec_key);
    Ok((sec_key, pub_key))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalletInfoFormatted {
    pub last_confirmed_height: u64,
    pub minimum_confirmations: u64,
    pub total: f64,
    pub amount_awaiting_finalization: f64,
    pub amount_awaiting_confirmation: f64,
    pub amount_immature: f64,
    pub amount_currently_spendable: f64,
    pub amount_locked: f64,
}

/*
    Get wallet info
    This contains wallet balances
*/
pub fn get_wallet_info(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    refresh_from_node: bool,
    min_confirmations: u64
) -> Result<WalletInfoFormatted, Error> {
    let api = Owner::new(wallet.clone(), None, None);

    match api.retrieve_summary_info(keychain_mask.as_ref(), refresh_from_node, min_confirmations) {
        Ok((_, wallet_summary)) => {
            Ok(WalletInfoFormatted {
                last_confirmed_height: wallet_summary.last_confirmed_height,
                minimum_confirmations: wallet_summary.minimum_confirmations,
                total: nano_to_deci(wallet_summary.total),
                amount_awaiting_finalization: nano_to_deci(wallet_summary.amount_awaiting_finalization),
                amount_awaiting_confirmation: nano_to_deci(wallet_summary.amount_awaiting_confirmation),
                amount_immature: nano_to_deci(wallet_summary.amount_immature),
                amount_currently_spendable: nano_to_deci(wallet_summary.amount_currently_spendable),
                amount_locked: nano_to_deci(wallet_summary.amount_locked)
            })
        }, Err(e) => {
            return  Err(e);
        }
    }

}

/*
    Recover wallet from mnemonic
*/
pub fn recover_from_mnemonic(mnemonic: &str, password: &str, config: &Config, name: &str) -> Result<(), Error> {
    let wallet = match get_wallet(&config) {
        Ok(conf) => {
            conf
        }
        Err(e) => {
            return  Err(e);
        }
    };
    let mut w_lock = wallet.lock();
    let lc = match w_lock.lc_provider() {
        Ok(wallet_lc) => {
            wallet_lc
        }
        Err(e) => {
            return  Err(e);
        }
    };

    //First check if wallet seed directory exists, if not create
    if let Ok(exists_wallet_seed) = lc.wallet_exists(None, None) {
        return if exists_wallet_seed {
            match lc.recover_from_mnemonic(
                ZeroingString::from(mnemonic), ZeroingString::from(password), None
            ) {
                Ok(_) => {
                    Ok(())
                }
                Err(e) => {
                    Err(e)
                }
            }
        } else {
            match lc.create_wallet(
                Some(&name),
                Some(ZeroingString::from(mnemonic)),
                32,
                ZeroingString::from(password),
                false,
                None,
            ) {
                Ok(_) => {
                    Ok(())
                }
                Err(e) => {
                    Err(e)
                }
            }
        }
    }
    Ok(())
}

/*
    Create a new wallet seed
*/
pub fn mnemonic() -> Result<String, mnemonic::Error> {
    let seed = create_seed(32);
    match mnemonic::from_entropy(&seed) {
        Ok(mnemonic_str) => {
            Ok(mnemonic_str)
        }, Err(e) => {
            return  Err(e);
        }
    }
}

fn create_seed(seed_length: u64) -> Vec<u8> {
    let mut seed: Vec<u8> = vec![];
    let mut rng = thread_rng();
    for _ in 0..seed_length {
        seed.push(rng.gen());
    }
    seed
}

/*
    Get wallet that will be used for calls to mwc wallet
*/
fn get_wallet(config: &Config) -> Result<Wallet, Error> {
    let wallet_config = match create_wallet_config(config.clone()) {
        Ok(conf) => {
            conf
        } Err(e) => {
            return Err(e);
        }
    };
    let target_chaintype = wallet_config.chain_type.unwrap_or(ChainTypes::Mainnet);
    if !global::GLOBAL_CHAIN_TYPE.is_init() {
        global::init_global_chain_type(target_chaintype)
    };
    if global::get_chain_type() != target_chaintype {
        global::set_local_chain_type(target_chaintype);
    };
    let node_api_secret = get_first_line(wallet_config.node_api_secret_path.clone());
    let node_client = HTTPNodeClient::new(vec![wallet_config.check_node_api_http_addr.clone()], node_api_secret).unwrap();
    let wallet =  match inst_wallet::<
        DefaultLCProvider<HTTPNodeClient, ExtKeychain>,
        HTTPNodeClient,
        ExtKeychain,
    >(wallet_config.clone(), node_client) {
        Ok(wallet_inst) => {
            wallet_inst
        }
        Err(e) => {
            return  Err(e);
        }
    };
    return Ok(wallet);
}
/*
    New wallet instance
*/
fn inst_wallet<L, C, K>(
    config: WalletConfig,
    node_client: C,
) -> Result<Arc<Mutex<Box<dyn WalletInst<'static, L, C, K>>>>, Error>
    where
        DefaultWalletImpl<'static, C>: WalletInst<'static, L, C, K>,
        L: WalletLCProvider<'static, C, K>,
        C: NodeClient + 'static,
        K: Keychain + 'static,
{
    let mut wallet = Box::new(DefaultWalletImpl::<'static, C>::new(node_client.clone()).unwrap())
        as Box<dyn WalletInst<'static, L, C, K>>;
    let lc = match wallet.lc_provider() {
        Ok(wallet_lc) => {
            wallet_lc
        }
        Err(err) => {
            return  Err(err);
        }
    };
    match lc.set_top_level_directory(&config.data_file_dir) {
        Ok(_) => {
            ()
        }
        Err(err) => {
            return  Err(err);
        }
    };
    Ok(Arc::new(Mutex::new(wallet)))
}

pub fn get_chain_height(config: &str) -> Result<u64, Error> {
    let config = match Config::from_str(&config.to_string()) {
        Ok(config) => {
            config
        }, Err(_e) => {
            return Err(Error::GenericError(format!(
                "{}",
                "Unable to get wallet config"
            )))
        }
    };
    let wallet_config = match create_wallet_config(config.clone()) {
        Ok(wallet_conf) => {
            wallet_conf
        }
        Err(e) => {
            return  Err(e);
        }
    };
    let node_api_secret = get_first_line(wallet_config.node_api_secret_path.clone());
    let node_client = HTTPNodeClient::new(vec![wallet_config.check_node_api_http_addr], node_api_secret)
        .map_err(|e| Error::ClientCallback(format!("{}", e)))?;
    let chain_tip = node_client.get_chain_tip()?;
    Ok(chain_tip.0)
}


/*

*/
pub fn wallet_scan_outputs(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    start_height: Option<u64>,
    number_of_blocks_to_scan: Option<u64>
) -> Result<String, Error> {
    let tip = {
        wallet_lock!(wallet, w);
        match w.w2n_client().get_chain_tip() {
            Ok(chain_tip) => {
                chain_tip.0
            },
            Err(_e) => {
                0
            }
        }
    };

    if tip == 0 {
        return Err(Error::GenericError(format!(
            "{}",
            "Unable to scan, could not determine chain height"
        )));
    }

    let start_height: u64 = match start_height {
        Some(h) => h,
        None => 1,
    };

    let number_of_blocks_to_scan: u64 = match number_of_blocks_to_scan {
        Some(h) => h,
        None => 0,
    };

    let last_block = start_height.clone() + number_of_blocks_to_scan;
    let end_height: u64 = match last_block.cmp(&tip) {
        Ordering::Less => {
            last_block
        },
        Ordering::Greater => {
            tip
        },
        Ordering::Equal => {
            last_block
        }
    };

    match scan(
        wallet.clone(),
        keychain_mask.as_ref(),
        false,
        start_height,
        end_height,
        &None,
        false,
        true
    ) {
        Ok(_) => {


            let parent_key_id = {
                wallet_lock!(wallet, w);
                w.parent_key_id().clone()
            };

            {
                wallet_lock!(wallet, w);
                let mut batch = match w.batch(keychain_mask.as_ref()) {
                    Ok(wallet_output_batch) => {
                        wallet_output_batch
                    }
                    Err(err) => {
                        return Err(err);
                    }
                };
                match batch.save_last_confirmed_height(&parent_key_id, end_height) {
                    Ok(_) => {
                        ()
                    }
                    Err(err) => {
                        return  Err(err);
                    }
                };
                match batch.commit() {
                    Ok(_) => {
                        ()
                    }
                    Err(err) => {
                        return  Err(err);
                    }
                }
            };


            let result = end_height;
            Ok(serde_json::to_string(&result).unwrap())
        }, Err(e) => {
            return  Err(e);
        }
    }
}


#[derive(Serialize, Deserialize)]
struct Strategy {
    selection_strategy_is_use_all: bool,
    total: u64,
    fee: u64,
}

/*
    Get transaction fees
    all possible Coin/Output selection strategies.
*/
pub fn tx_strategies(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    amount: u64,
    minimum_confirmations: u64,
) -> Result<String, Error> {

    let mut result = vec![];
    wallet_lock!(wallet, w);

    let args = InitTxArgs {
        src_acct_name: None,
        amount,
        minimum_confirmations,
        max_outputs: 500,
        num_change_outputs: 1,
        estimate_only: Some(true),
        message: None,
        ..Default::default()
    };

    match owner::init_send_tx(&mut **w, keychain_mask.as_ref(), &args, true, 1) {
        Ok(slate) => {
            result.push(Strategy {
                selection_strategy_is_use_all: false,
                total: slate.amount,
                fee: slate.fee,

            });
        }, Err(e) => {
            return Err(e);
        }
    }

    Ok(serde_json::to_string(&result).unwrap())
}

pub fn txs_get(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    refresh_from_node: bool,
) -> Result<String, Error> {
    let api = Owner::new(wallet.clone(), None, None);
    let txs = match api.retrieve_txs(
        keychain_mask.as_ref(),
        refresh_from_node,
        None,
        None,
        None
    ) {
        Ok((_, tx_entries)) => {
            tx_entries
        }, Err(e) => {
            return  Err(e);
        }
    };

    let result = txs;
    Ok(serde_json::to_string(&result).unwrap())
}

/*
    Init tx as sender
*/
pub fn tx_create(
    wallet: &Wallet,
    keychain_mask: Option<SecretKey>,
    amount: u64,
    minimum_confirmations: u64,
    selection_strategy_is_use_all: bool,
    _mwcmqs_config: &str,
    address: &str,
    note: &str,
) -> Result<String, Error> {
    let owner_api = Owner::new(wallet.clone(), None, None);
    //let mwcmqs_conf = serde_json::from_str::<MQSConfig>(mwcmqs_config).unwrap();

    let message = match note {
        "" => None,
        _ => Some(note.to_string()),
    };

    let init_send_args = InitTxSendArgs {
        method: "mwcmqs".to_string(),
        dest: address.to_string(),
        apisecret: None,
        finalize: true,
        post_tx: true,
        fluff: true
    };
    
    let args = InitTxArgs {
        src_acct_name: Some("default".to_string()),
        amount,
        minimum_confirmations,
        max_outputs: 500,
        num_change_outputs: 1,
        selection_strategy_is_use_all,
        send_args: Some(init_send_args),
        message: message,
        ..Default::default()
    };


    match owner_api.init_send_tx(keychain_mask.as_ref(), &args, 1) {
        Ok(slate)=> {
            debug!("SLATE SEND RESPONSE IS  {:?}", slate);
            //Get transaction for the slate, we will use type to determing if we should finalize or receive tx
            let txs = match owner_api.retrieve_txs(
                keychain_mask.as_ref(),
                false,
                None,
                Some(slate.id),
                None
            ) {
                Ok(txs_result) => {
                    txs_result
                }, Err(e) => {
                    return Err(e);
                }
            };
            let final_result = (
                serde_json::to_string(&txs.1).unwrap(),
                serde_json::to_string(&slate).unwrap()
            );
            let str_result = serde_json::to_string(&final_result).unwrap();
            Ok(str_result)
        },
        Err(e)=> {
            return Err(e);
        }
    }
}

/*
    Cancel tx by id
*/
pub fn tx_cancel(wallet: &Wallet, keychain_mask: Option<SecretKey>, tx_slate_id: Uuid) -> Result<String, Error> {
    let api = Owner::new(wallet.clone(), None, None);
    match  api.cancel_tx(keychain_mask.as_ref(), None, Some(tx_slate_id)) {
        Ok(_) => {
            Ok("cancelled".to_owned())
        },Err(e) => {
            return Err(e);
        }
    }
}

/*
    Get transaction by slate id
*/
pub fn tx_get(wallet: &Wallet, refresh_from_node: bool, tx_slate_id: &str) -> Result<String, Error> {
    let api = Owner::new(wallet.clone(), None, None);
    let uuid = Uuid::parse_str(tx_slate_id).map_err(|e| MWCWalletControllerError::GenericError(e.to_string())).unwrap();
    let txs = api.retrieve_txs(None, refresh_from_node, None, Some(uuid), None).unwrap();
    Ok(serde_json::to_string(&txs.1).unwrap())
}

pub fn convert_deci_to_nano(amount: f64) -> u64 {
    let base_nano = 1000000000;
    let nano = amount * base_nano as f64;
    nano as u64
}

pub fn nano_to_deci(amount: u64) -> f64 {
    let base_nano = 1000000000;
    let decimal = amount as f64 / base_nano as f64;
    decimal
}

/*

*/
pub fn open_wallet(config_json: &str, password: &str) -> Result<(Wallet, Option<SecretKey>), Error> {
    let config = match Config::from_str(&config_json.to_string()) {
        Ok(config) => {
            config
        }, Err(_e) => {
            return Err(Error::GenericError(format!(
                "{}",
                "Unable to get wallet config"
            )))
        }
    };
    let wallet = match get_wallet(&config) {
        Ok(wllet) => {
            wllet
        }
        Err(err) => {
            return  Err(err);
        }
    };
    let mut secret_key = None;
    let mut opened = false;
    {
        let mut wallet_lock = wallet.lock();
        let lc = wallet_lock.lc_provider()?;
        if let Ok(exists_wallet) = lc.wallet_exists(None, None) {
            if exists_wallet {
                let temp = match lc.open_wallet(
                    None,
                    ZeroingString::from(password),
                    false,
                    false,
                    None) {
                    Ok(tmp_key) => {
                        tmp_key
                    }
                    Err(err) => {
                        return Err(err);
                    }
                };
                secret_key = temp;
                let wallet_inst = match lc.wallet_inst() {
                    Ok(wallet_backend) => {
                        wallet_backend
                    }
                    Err(err) => {
                        return Err(err);
                    }
                };
                if let Some(account) = config.account {
                    match wallet_inst.set_parent_key_id_by_name(&account) {
                        Ok(_) => {
                            ()
                        }
                        Err(err) => {
                            return  Err(err);
                        }
                    }
                    opened = true;
                }
            }
        }
    }
    if opened {
        Ok((wallet, secret_key))
    } else {
        Err(Error::WalletSeedDoesntExist)
    }
}


pub fn close_wallet(wallet: &Wallet) -> Result<String, Error> {
    let mut wallet_lock = wallet.lock();
    let lc = wallet_lock.lc_provider()?;
    match lc.wallet_exists(None, None)? {
        true => {
            lc.close_wallet(None)?
        }
        false => {
            return Err(
                Error::WalletSeedDoesntExist
            );
        }
    }
    Ok("Wallet has been closed".to_owned())
}

pub fn validate_address(str_address: &str) -> bool {
    match MWCMQSAddress::from_str(str_address) {
        Ok(addr) => {
            if addr.address_type() == AddressType::MWCMQS {
                return true;
            }
            false
        }
        Err(_) => {
            false
        }
    }
}

pub fn delete_wallet(config: Config) -> Result<String, Error> {
    let mut result = String::from("");
    // get wallet object in order to use class methods
    let wallet = match get_wallet(&config) {
        Ok(wllet) => {
            wllet
        }
        Err(e) => {
            return  Err(e);
        }
    };
    //First close the wallet
    if let Ok(_) = close_wallet(&wallet) {
        let api = Owner::new(wallet.clone(), None, None);
        match api.delete_wallet(None) {
            Ok(_) => {
                result.push_str("deleted");
            }
            Err(err) => {
                return  Err(err);
            }
        };
    } else {
        return Err(
            Error::GenericError(format!("{}", "Error closing wallet"))
        );
    }
    Ok(result)
}


#[derive(Debug, Clone)]
pub struct Listener {
    pub wallet_ptr_str: String,
    // pub wallet_data: (i64, Option<SecretKey>),
    pub mwcmqs_config: String
}


impl Task for Listener {
    type Output = usize;

    fn run(&self, _cancel_tok: &CancellationToken) -> Result<Self::Output, anyhow::Error> {
        let spins = 0;
        let tuple_wallet_data: (i64, Option<SecretKey>) = serde_json::from_str(&self.wallet_ptr_str)?;
        let wlt = tuple_wallet_data.0;
        let sek_key = tuple_wallet_data.1;
        let mwcmqs_conf = serde_json::from_str::<MQSConfig>(&self.mwcmqs_config)?;

        unsafe {
            ensure_wallet!(wlt, wallet);
            if mwc_wallet_impls::adapters::get_mwcmqs_brocker().is_none() {
                let mwcmqs_broker = controller::init_start_mwcmqs_listener(
                    wallet.clone(),
                    mwcmqs_conf.clone(),
                    Arc::new(Mutex::new(sek_key.clone())),
                    true,
                );
                match mwcmqs_broker {
                    Ok((_, _subsriber)) => {
                        println!("MWCMQS listener started successfully.");
                    }
                    Err(e) => {
                        println!("Error starting MWCMQS listener: {}", e);
                    }
                }
            }
        }
        Ok(spins)
    }
}

export_task! {
    Task: Listener;
    spawn: listener_spawn;
    wait: listener_wait;
    poll: listener_poll;
    cancel: listener_cancel;
    cancelled: listener_cancelled;
    handle_destroy: listener_handle_destroy;
    result_destroy: listener_result_destroy;
}

#[no_mangle]
pub unsafe extern "C" fn rust_mwcmqs_listener_start(
    wallet: *const c_char,
    mwcmqs_config: *const c_char,
) -> *mut c_void {
    let wallet_ptr = CStr::from_ptr(wallet);
    let mwcmqs_config = CStr::from_ptr(mwcmqs_config);
    let mwcmqs_config = mwcmqs_config.to_str().unwrap();

    let wallet_data = wallet_ptr.to_str().unwrap();
    let listen = Listener {
        wallet_ptr_str: wallet_data.to_string(),
        mwcmqs_config: mwcmqs_config.parse().unwrap()
    };

    let handler = listener_spawn(&listen);
    let handler_value = handler.read();
    let boxed_handler = Box::new(handler_value);
    Box::into_raw(boxed_handler) as *mut _
}

#[no_mangle]
pub unsafe extern "C" fn _listener_cancel(handler: *mut c_void) -> *const c_char {
    let handle = handler as *mut TaskHandle<usize>;
    listener_cancel(handle);
    if let Some((_, mut subscriber)) = mwc_wallet_impls::adapters::get_mwcmqs_brocker() {
        if subscriber.is_running() {
            if subscriber.stop() {
                println!("MWCMQS listener stopped successfully.");
            } else {
                println!("Unable to stop the MWCMQS listener.");
            }   
        }
    }
    let error_msg = format!("{}", listener_cancelled(handle));
    let error_msg_ptr = CString::new(error_msg).unwrap();
    let ptr = error_msg_ptr.as_ptr();
    std::mem::forget(error_msg_ptr);
    ptr
}