#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]

extern crate paho_mqtt;
extern crate base64;
extern crate hmac_sha256;
use paho_mqtt as mqtt;

use std::env;
use std::slice;
use std::ptr;
use std::mem;
use std::ffi::CStr;
use std::string;
use std::process;
use std::thread;
use std::time;
use std::time::{SystemTime, UNIX_EPOCH};

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

const VERSION: &str = "1.0";
const ENV_CONNECTION_STRING: &str = "AZ_IOT_CONNECTION_STRING";

fn get_config_value(display_name: &str, 
                    config_name: &str, 
                    default_value: Option<&str>, 
                    required: bool, 
                    display: bool) -> Result<String, az_result_core> {

    print!("{} = ", display_name);

    match env::var(config_name) {
        Ok(value) => {
            if display {
                println!("{}", value);
            }
            else {
                println!("***");
            }
            return Ok(value);
        },
        Err(e) => {
            match default_value {
                Some(default) => {
                    if display {
                        println!("{}", default);
                    }
                    else {
                        println!("***");
                    }
                    return Ok(default.to_string());
                },
                None => {
                    println!("failed {}", e);
                    return Err(az_result_core_AZ_ERROR_ARG);
                }
            }
        },
    }
}

fn split_connection_string(connection_string: &String) -> Result<(String, String, String), az_result_core> {

    let parts: Vec<&str> = connection_string.split(';').collect();
    let mut device_id: String = String::new();
    let mut host_name: String = String::new();
    let mut sas_key: String = String::new();

    let deviceid: &str = "deviceid=";
    let hostname: &str = "hostname=";
    let sharedaccesskey: &str = "sharedaccesskey=";

    for (i, part) in parts.iter().enumerate() {
        if part.to_ascii_lowercase().starts_with(deviceid) {
            device_id = part[deviceid.len()..].to_string();
        }
        else if part.to_ascii_lowercase().starts_with(hostname) {
            host_name = part[hostname.len()..].to_string();
        }
        else if part.to_ascii_lowercase().starts_with(sharedaccesskey) {
            sas_key = part[sharedaccesskey.len()..].to_string();
        }
    }

    if device_id.is_empty() || host_name.is_empty() || sas_key.is_empty() {
        return Err(az_result_core_AZ_ERROR_ARG);
    }

    Ok((host_name, device_id, sas_key))
}

struct Config {
    connection_string: String,
    device_id: String,
    host_name: String,
    sas_key: String,
    decoded_sas_key: Vec<u8>,
    port: i32,
    ttl: u32,
    mqtt_client_id: String,
    mqtt_user_id: String,
    mqtt_publish_topic: String,
}

fn get_empty_span() -> az_span {
    let result: az_span = az_span {
        _internal: az_span__bindgen_ty_1 {
            ptr: ptr::null_mut(),
            size: 0,
        }
    };
    result
}

fn get_span_out_from_vector(v: &mut Vec<u8>) -> az_span {
    let result: az_span = az_span {
        _internal: az_span__bindgen_ty_1 {
            ptr: v.as_mut_ptr(),
            size: v.capacity() as i32,
        }
    };
    result
}

fn get_span_in_from_vector(v: &mut Vec<u8>) -> az_span {
    let result: az_span = az_span {
        _internal: az_span__bindgen_ty_1 {
            ptr: v.as_mut_ptr(),
            size: v.len() as i32,
        }
    };
    result
}

fn get_span_in_from_string(s: &mut String) -> az_span {
    let result: az_span = az_span {
        _internal: az_span__bindgen_ty_1 {
            ptr: s.as_mut_ptr(),
            size: s.len() as i32,
        }
    };
    result
}

fn get_span_size(span: az_span) -> i32 {
    span._internal.size
}

fn az_func_wrapper(rc: az_result_core) -> Result<az_result_core,az_result_core> {
    if rc == az_result_core_AZ_OK {
        Ok(rc)
    }
    else {
        Err(rc)
    }
}

fn get_password(client: &az_iot_hub_client, config: &Config) -> Result<String, az_result_core> {
    
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH).expect("Could not get time").as_secs() + config.ttl as u64;
    let mut signature_vector: Vec<u8> = Vec::with_capacity(200);
    let signature = get_span_out_from_vector(&mut signature_vector);
    let mut work = get_empty_span();
    let null_span = get_empty_span();
    let mut length_out: Vec<u64> = [ 99 ].to_vec();

    if let Err(err) = az_func_wrapper(unsafe { az_iot_hub_client_sas_get_signature(client,
            epoch,
            signature,
            &mut work) }) {
        println!("Failed to get signature {}", err);
        return Err(err);
    }

    unsafe { signature_vector.set_len(get_span_size(work) as usize) };

    let binpw = hmac_sha256::HMAC::mac(&signature_vector, &config.decoded_sas_key);
    let mut sas = base64::encode(binpw);
    println!("sas={}", sas);
    let sas_span = get_span_in_from_string(&mut sas);
    let mut mqtt_password: Vec<i8> = Vec::with_capacity(200);

    if let Err(err) = az_func_wrapper(unsafe { az_iot_hub_client_sas_get_password(client,
            epoch,
            sas_span,
            null_span,
            mqtt_password.as_mut_ptr(),
            mqtt_password.capacity() as u64,
            length_out.as_mut_ptr()) }) {
        println!("Failed to get password {}", err);
        return Err(err);
    }

    let mut mqtt_password = std::mem::ManuallyDrop::new(mqtt_password);
    let u8password = unsafe { Vec::from_raw_parts(mqtt_password.as_mut_ptr() as *mut u8 ,length_out[0] as usize, mqtt_password.capacity()) };

    let ret = String::from_utf8_lossy(&u8password);

    Ok(ret.to_string())
}

fn main() {
    println!("Azure SDK for C IoT device sample in Rust: {}", VERSION);

    let mut config: Config = Config {
        connection_string: String::new(),
        device_id: String::new(),
        host_name: String::new(),
        sas_key: String::new(),
        decoded_sas_key: Vec::new(),
        port: 8883,
        ttl: 3600,
        mqtt_client_id: String::new(),
        mqtt_user_id: String::new(),
        mqtt_publish_topic: String::new(),
    };

    config.connection_string = get_config_value("Connection String", ENV_CONNECTION_STRING, Option::None, true, false)
        .expect("Failed to retrieve connection string");

    let parts = split_connection_string(&config.connection_string)
        .expect("Missing or invalid connection string");
    
    config.host_name = parts.0;
    config.device_id = parts.1;
    let sas_key = parts.2;

    let mut client: az_iot_hub_client = az_iot_hub_client {
        _internal: az_iot_hub_client__bindgen_ty_1 {
            iot_hub_hostname: get_empty_span(),
            device_id: get_empty_span(),
            options: az_iot_hub_client_options {
                module_id:  get_empty_span(),
                user_agent:  get_empty_span(),
                model_id:  get_empty_span(),
            }
        }
    };

    //let mut rc: az_result_core; // = az_result_core_AZ_OK;
    let options: az_iot_hub_client_options  = unsafe { az_iot_hub_client_options_default() };

    if let Err(err) = az_func_wrapper(unsafe { az_iot_hub_client_init(&mut client, 
            get_span_in_from_string(&mut config.host_name),
            get_span_in_from_string(&mut config.device_id),
            &options) }) {
        println!("Failed to initialize client: {}", err);
        process::exit(4);
    }

    let mut work_buff: Vec<u8> = Vec::with_capacity(200);
    let mut work_buff_size: Vec<u64> = [ 99 ].to_vec();

    if let Err(err) = az_func_wrapper(unsafe { az_iot_hub_client_get_client_id(&client, 
            work_buff.as_mut_ptr() as *mut i8, 
            work_buff.capacity() as u64, 
            work_buff_size.as_mut_ptr()) }) {
        println!("Failed to get MQTT client id: {}", err);
        process::exit(4);
    }

    unsafe { work_buff.set_len(work_buff_size[0] as usize) };
    config.mqtt_client_id = String::from_utf8_lossy(&work_buff).to_string();
    println!("mqtt_client_id={}", config.mqtt_client_id);

    if let Err(err) = az_func_wrapper(unsafe { az_iot_hub_client_get_user_name(&client, 
            work_buff.as_mut_ptr() as *mut i8, 
            work_buff.capacity() as u64, 
            work_buff_size.as_mut_ptr()) }) {
        println!("Failed to get MQTT user id: {}", err);
        process::exit(4);
    }

    unsafe { work_buff.set_len(work_buff_size[0] as usize) };
    config.mqtt_user_id = String::from_utf8_lossy(&work_buff).to_string();
    println!("mqtt_user_id={}", config.mqtt_user_id);

    config.decoded_sas_key = base64::decode(&sas_key)
        .expect("Failed to decode SAS key");

    if let Err(err) = az_func_wrapper(unsafe { az_iot_hub_client_telemetry_get_publish_topic(&client,
            ptr::null_mut(), work_buff.as_mut_ptr() as *mut i8, 
            work_buff.capacity() as u64, 
            work_buff_size.as_mut_ptr()) }) {
        println!("Failed to get publish topic: {}", err);
        process::exit(4);
    }

    unsafe { work_buff.set_len(work_buff_size[0] as usize) };
    config.mqtt_publish_topic = String::from_utf8_lossy(&work_buff).to_string();
    println!("mqtt_publish_topic={}", config.mqtt_publish_topic);

    let mut password: String = "".to_string();
    match get_password(&client, &config) {
        Err(err) => {
            print!("Failed to generate password {}", err);
        }
        Ok(pw) => password = pw,
    }

    println!("password={}", password);

    let uri = "ssl://".to_string() + &config.host_name + ":" + &config.port.to_string();
    println!("uri={}", uri);

    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri(uri)
        .client_id(config.mqtt_client_id)
        .persistence(mqtt::PersistenceType::None)
        .finalize();

    let mut pwd = env::current_dir().expect("Could not get current directory");
    pwd.push("BaltimoreCyberTrust.pem");

    let ssl_opts = mqtt::SslOptionsBuilder::new()
        .trust_store(pwd.to_str().expect("Could not decipher returned path to current directory"))
        .finalize();

    let connect_opts = mqtt::ConnectOptionsBuilder::new()
        .user_name(config.mqtt_user_id)
        .password(password)
        .ssl_options(ssl_opts)
        .automatic_reconnect(time::Duration::new(1, 0), time::Duration::new(60 * 60, 0))
        .finalize();

    let mqtt_client = mqtt::Client::new(create_opts).expect("Failed to create client");

    if let Err(e) = mqtt_client.connect(connect_opts) {
        println!("Cannot connect: {:?}", e);
        process::exit(4);
    }

    let mut message: mqtt::Message;

    for i in 0..30 {
        message = mqtt::MessageBuilder::new()
            .topic(&config.mqtt_publish_topic)
            .payload(format!("Rust Message #{}", i))
            .qos(1)
            .finalize();

        print!("Sending message {}: ", i);
        match mqtt_client.publish(message) {
            Ok(n) => println!("Success"),
            Err(err) => {
                println!("Send failed: {}", err);
                process::exit(4);
            }
        }
        thread::sleep(time::Duration::from_millis(1000));
    }

    mqtt_client.disconnect(mqtt::DisconnectOptions::new()).expect("Failed to disconnect");
}
