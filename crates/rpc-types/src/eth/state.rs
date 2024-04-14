//! bindings for state overrides in eth_call

use alloy_primitives::{Address, Bytes, B256, U256, U64};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

/// A set of account overrides
pub type StateOverride = HashMap<Address, AccountOverride>;

/// Custom account override used in call
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountOverride {
    /// Fake balance to set for the account before executing the call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance: Option<U256>,
    /// Fake nonce to set for the account before executing the call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce: Option<U64>,
    /// Fake EVM bytecode to inject into the account before executing the call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Bytes>,
    /// Fake key-value mapping to override all slots in the account storage before executing the
    /// call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<HashMap<B256, U256>>,
    /// Fake key-value mapping to override individual slots in the account storage before executing
    /// the call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_diff: Option<HashMap<B256, U256>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;

    #[test]
    #[should_panic(expected = "invalid type")]
    fn test_invalid_json_structure() {
        let invalid_json = r#"{
            "0x1234567890123456789012345678901234567890": {
                "balance": true
            }
        }"#;

        let _: StateOverride = serde_json::from_str(invalid_json).unwrap();
    }

    #[test]
    fn test_large_values_in_override() {
        let large_values_json = r#"{
            "0x1234567890123456789012345678901234567890": {
                "balance": "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "nonce": "0xffffffffffffffff"
            }
        }"#;

        let state_override: StateOverride = serde_json::from_str(large_values_json).unwrap();
        let acc =
            state_override.get(&address!("1234567890123456789012345678901234567890")).unwrap();
        assert_eq!(acc.balance, Some(U256::MAX));
        assert_eq!(acc.nonce, Some(U64::MAX));
    }

    #[test]
    fn test_default_account_override() {
        let acc_override = AccountOverride::default();
        assert!(acc_override.balance.is_none());
        assert!(acc_override.nonce.is_none());
        assert!(acc_override.code.is_none());
        assert!(acc_override.state.is_none());
        assert!(acc_override.state_diff.is_none());
    }

    #[test]
    fn test_state_override() {
        let s = r#"{
            "0x0000000000000000000000000000000000000124": {
                "code": "0x6080604052348015600e575f80fd5b50600436106026575f3560e01c80632096525514602a575b5f80fd5b60306044565b604051901515815260200160405180910390f35b5f604e600242605e565b5f0360595750600190565b505f90565b5f82607757634e487b7160e01b5f52601260045260245ffd5b50069056fea2646970667358221220287f77a4262e88659e3fb402138d2ee6a7ff9ba86bae487a95aa28156367d09c64736f6c63430008140033"
            }
        }"#;
        let state_override: StateOverride = serde_json::from_str(s).unwrap();
        let acc =
            state_override.get(&address!("0000000000000000000000000000000000000124")).unwrap();
        assert!(acc.code.is_some());
    }
    #[test]
    fn test_state_override_state_diff() {
        let s = r#"{
                "0x1b5212AF6b76113afD94cD2B5a78a73B7d7A8222": {
                    "balance": "0x39726378b58c400000",
                    "stateDiff": {}
                },
                "0xdAC17F958D2ee523a2206206994597C13D831ec7": {
                    "stateDiff": {
                        "0xede27e4e7f3676edbf125879f17a896d6507958df3d57bda6219f1880cae8a41": "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
                    }
                }
            }"#;
        let state_override: StateOverride = serde_json::from_str(s).unwrap();
        let acc =
            state_override.get(&address!("1b5212AF6b76113afD94cD2B5a78a73B7d7A8222")).unwrap();
        assert!(acc.state_diff.is_some());
    }
}
