// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub struct ValidatorDetails<'a> {
    // it is assumed those values are always valid since they're being provided in our defaults file
    pub nymd_urls: &'a [&'a str],
    pub api_urls: &'a [&'a str],
}

impl<'a> ValidatorDetails<'a> {
    pub const fn new(nymd_urls: &'a [&'a str], api_urls: &'a [&'a str]) -> Self {
        ValidatorDetails {
            nymd_urls,
            api_urls,
        }
    }
}

pub const DEFAULT_VALIDATORS: &[ValidatorDetails] = &[ValidatorDetails::new(
    &[
        "https://testnet-milhon-validator1.nymtech.net",
        "https://testnet-milhon-validator2.nymtech.net",
    ],
    &["https://testnet-milhon-validator1.nymtech.net/api"],
)];

pub const DEFAULT_VALIDATOR_REST_ENDPOINTS: &[&str] = &[
    "http://testnet-milhon-validator1.nymtech.net:1317",
    "http://testnet-milhon-validator2.nymtech.net:1317",
];
pub const DEFAULT_MIXNET_CONTRACT_ADDRESS: &str = "punk10pyejy66429refv3g35g2t7am0was7yalwrzen";
pub const NETWORK_MONITOR_ADDRESS: &str = "punk1v9qauwdq5terag6uvfsdytcs2d0sdmfdy7hgk3";

/// Defaults Cosmos Hub/ATOM path
pub const COSMOS_DERIVATION_PATH: &str = "m/44'/118'/0'/0/0";
pub const BECH32_PREFIX: &str = "punk";
pub const DENOM: &str = "upunk";
// as set by validators in their configs
// (note that the 'amount' postfix is relevant here as the full gas price also includes denom)
pub const GAS_PRICE_AMOUNT: f64 = 0.025;

pub const DEFAULT_MIX_LISTENING_PORT: u16 = 1789;

// 'GATEWAY'
pub const DEFAULT_CLIENT_LISTENING_PORT: u16 = 9000;

// 'MIXNODE'
pub const DEFAULT_VERLOC_LISTENING_PORT: u16 = 1790;
pub const DEFAULT_HTTP_API_LISTENING_PORT: u16 = 8000;
