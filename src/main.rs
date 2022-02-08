// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2022 Robert Mascaro

mod macros;

use clap::Parser;
use futures::executor::block_on;
use std::error::Error;
use std::time::Duration;

use serde::Deserialize;
use serde::Deserializer;
use serde_json;
use std::{
    fs::File,
    io::{prelude::*, BufReader},
    path::Path,
};

#[derive(Debug)]
struct StrError<'a>(&'a str);

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(name = "fzvoid")]
#[clap(author = "Robert Mascaro")]
#[clap(version = "1.0")]
#[clap(about = "Void a Fat Zebra transaction", long_about = None)]
#[derive(Clone)]
struct Cli {
    /// The Fat Zebra merchant username
    #[clap(short, long)]
    username: String,
    /// The API Token
    #[clap(short, long)]
    token: String,
    /// The purchase reference
    #[clap(short, long)]
    reference: Option<String>,
    /// The upload filename
    #[clap(short, long)]
    filename: Option<String>,
}

#[derive(Debug)]
struct Params {
    username: String,
    token: String,
    reference: String,
    filename: String,
}

struct Url {
    sandbox_fetch_url: String,
    production_fetch_url: String,
    sandbox_void_url: String,
    production_void_url: String,
}

#[derive(Deserialize, Default, Debug)]
#[serde(default)]
struct FetchResponses {
    successful: bool,
    #[serde(deserialize_with = "deserialize_optional_field")]
    #[serde(skip_serializing_if = "Option::is_none")]
    response: Option<Option<FetchResponse>>,
    #[serde(deserialize_with = "deserialize_optional_field")]
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<Option<FetchErrors>>,
}

#[derive(Deserialize, Default, Debug)]
struct FetchResponse {
    //successful: bool,
    id: String,
    //reference: String,
}

#[derive(Deserialize, Default, Debug)]
#[serde(transparent)]
struct FetchErrors {
    errors: Vec<String>,
}

fn deserialize_optional_field<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    //Ok(Some(Option::deserialize(deserializer)?))
    let _result: Option<T> = match Option::deserialize(deserializer) {
        Ok(_result) => match _result {
            Some(r) => return Ok(r),
            _ => return Ok(None),
        },
        Err(_e) => {
            return Ok(None);
        }
    };
}

impl FetchResponses {
    async fn fetch_purchase(_args: &Params, refx: &String) -> Result<FetchResponses, Box<dyn Error>> {
        let mut auth_str = String::new();
        auth_str.push_str(&_args.username);
        auth_str.push(':');
        auth_str.push_str(&_args.token);

        let auth = base64::encode(auth_str);

        let client = reqwest::Client::new();
        let http_response = client
            .get(Url::new().get_fetch_url(&_args.username) + &refx)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("Authorization", "Basic ".to_owned() + &auth)
            .timeout(Duration::from_secs(10))
            .send()
            .await?
            .text()
            .await
            .unwrap();

        let r: FetchResponses = match serde_json::from_str(http_response.as_str()) {
            Ok(r) => r,
            Err(_) => {
                return return_error("00Error voiding transaction: ", &refx);
            }
        };

        Ok(r)
    }

    async fn void_transaction(
        _args: &Params,
        refx: &String,
        id: String,
    ) -> Result<FetchResponses, Box<dyn Error>> {
        let mut auth_str = String::new();
        auth_str.push_str(&_args.username);
        auth_str.push(':');
        auth_str.push_str(&_args.token);

        let auth = base64::encode(auth_str);

        let client = reqwest::Client::new();
        let http_response = client
            .post(Url::new().get_void_url(&_args.username) + &id)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("Authorization", "Basic ".to_owned() + &auth)
            .timeout(Duration::from_secs(10))
            .send()
            .await?
            .text()
            .await?;

        match serde_json::from_str(http_response.as_str()) {
            Ok(r) => {
                let b: FetchResponses = r;
                //p!(b);
                if b.successful {
                    println!("{} - Voided",&refx);
                    return Ok(b);
                } else {
                    println!("{} - Voiding failed - {:?}",&refx,b.errors.unwrap().unwrap().errors.first().unwrap());
                    return return_error("01Error voiding transaction: ", &refx);
                }
            }
            Err(_r) => {
                return return_error("02Error voiding transaction: ", &refx);     
            }
        };
    }
}

impl Default for Url {
    fn default() -> Self {
        Url {
            sandbox_fetch_url: "https://gateway.pmnts-sandbox.io/v1.0/purchases/".to_string(),
            production_fetch_url: "https://gateway.pmnts.io/v1.0/purchases/".to_string(),
            sandbox_void_url: "https://gateway.pmnts-sandbox.io/v1.0/purchases/void?id="
                .to_string(),
            production_void_url: "https://gateway.pmnts.io/v1.0/purchases/void?id=".to_string(),
        }
    }
}

impl Url {
    fn new() -> Self {
        return Self {
            ..Default::default()
        };
    }

    fn get_fetch_url(self, merchant_id: &String) -> String {
        match merchant_id.as_str() {
            "SC-scnet" => self.sandbox_fetch_url,
            "TEST" => self.sandbox_fetch_url,
            _ => self.production_fetch_url,
        }
    }

    fn get_void_url(self, merchant_id: &String) -> String {
        match merchant_id.as_str() {
            "SC-scnet" => self.sandbox_void_url,
            "TEST" => self.sandbox_void_url,
            _ => self.production_void_url,
        }
    }
}

impl Params {
    fn new() -> Self {
        return Self {
            ..Default::default()
        };
    }
}

impl Default for Params {
    fn default() -> Self {
        Params {
            username: String::new(),
            token: String::new(),
            reference: String::new(),
            filename: String::new(),
        }
    }
}

fn return_error<T>(msg: &str, reference: &String) -> Result<T, Box<dyn Error>> 
{
    let mut err_str = String::new();
    err_str.push_str(msg);
    err_str.push_str(&reference);
    return Err(err_str.into())
}

fn read_file(filename: impl AsRef<Path>) -> Vec<String> {
    match File::open(filename).map_err(|_| "Please specify a valid file name") {
        Ok(file) => {
            let buf = BufReader::new(file);
            return buf.lines()
            .map(|l| l.expect("Could not parse line"))
            .collect();
        },
        Err(_) => return vec![]
    };
}

fn fetch_n_void(_params: &Params,reference: &Option<&String>) -> Result<(), Box<dyn Error>> {

    let mut refx = &_params.reference;

    if let Some(v) = reference {
        refx = v;
    }

    let future_purchase = FetchResponses::fetch_purchase(&_params, &refx);

    //Not sure why this is needed works anyway - so comment out for now
    //let handle = tokio::runtime::Handle::current();
    //handle.enter();
    if let Ok(fetch_response) = block_on(future_purchase) {
        let fe = fetch_response;
        if fe.successful {
            let future_void =
                FetchResponses::void_transaction(&_params, &refx, fe.response.unwrap().unwrap().id);
            if let Ok(r) = block_on(future_void) {
                println!("{} - Voiding failed - {:?}",&refx,r.errors.unwrap().unwrap().errors.first().unwrap());
                return Ok(());
            } else {
                return_error("Error voiding transaction: ", refx)
            }
        } else {
            println!("{} - Voiding failed - {:?}",&refx,fe.errors.unwrap().unwrap().errors.first().unwrap());
            return_error("Could not fetch transaction: ", refx)
        }
    } else {
        return_error("Error fetching transaction: ", refx)
    }

}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //Parse the commandline
    let _args = Cli::parse();

    //Populate cli optionals
    let mut _params = Params::new();

    match (_args.filename, _args.reference) {
        (Some(filename), None) => {
            _params.username = _args.username;
            _params.token = _args.token;
            _params.filename = filename.to_string();
            _params.reference = String::new();
        }
        (None, Some(reference)) => {
            _params.username = _args.username;
            _params.token = _args.token;
            _params.filename = String::new();
            _params.reference = reference.to_string();
        }
        (Some(filename), Some(_)) => {
            _params.username = _args.username;
            _params.token = _args.token;
            _params.filename =filename.to_string();
            _params.reference =  String::new();
        }
        _ => (),
    }

    if _params.filename.len() == 0 {
        fetch_n_void(&_params,&None)
        
    } else {
        let void_trxs = read_file(&_params.filename);
        if void_trxs.is_empty() { 
            return return_error("Error opening file: ", &"please check file and path".to_string()); 
        }
        for line in void_trxs {
            let _ = fetch_n_void(&_params,&Some(&line));
        }
        Ok(())
    }
}
