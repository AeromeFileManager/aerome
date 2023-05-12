/*
 * Copyright (c) 2023 Jesse Tuchsen
 *
 * This file is part of Aerome.
 *
 * Aerome is free software: you can redistribute it and/or modify it under the terms of the GNU
 * General Public License as published by the Free Software Foundation, version 3 of the License.
 *
 * Aerome is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even
 * the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General
 * Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with Aerome. If not, see
 * <https://www.gnu.org/licenses/>.
 */

use std::fs;
use dirs;
use cozo::{self,Db,SqliteStorage,DataValue};
use crate::constants::APP_NAME;
use crate::models::{Account,AccountDirect,AccountAerome,Settings};
use std::collections::BTreeMap;

pub struct Store {
    db: Db<SqliteStorage>
}

impl Store {
    pub fn new() -> Self {
        let data_dir = dirs::data_dir().unwrap().join(APP_NAME);
        let store_file = data_dir.join("store.db");

        if !data_dir.exists() {
            fs::create_dir_all(data_dir).unwrap();
        }

        let store_exists = store_file.exists();
        let db = cozo::new_cozo_sqlite(store_file).expect("Could not create database store");

        Store { db }
    }

    pub fn set_account(&self, account: &Option<Account>) {
        let params = match account {
            None => vec![
                ("type", DataValue::Str("".into())),
                ("active", false.into()),
                ("key", DataValue::Str("".into())),
                ("email", DataValue::Str("".into()))
            ],
            Some(Account::Direct(account)) => vec![
                ("type", DataValue::Str("direct".into())),
                ("active", true.into()),
                ("key", DataValue::Str(account.0.clone().into())),
                ("email", DataValue::Str("".into()))
            ],
            Some(Account::Aerome(account)) => vec![
                ("type", DataValue::Str("aerome".into())),
                ("active", account.active.into()),
                ("key", DataValue::Str(account.key.clone().into())),
                ("email", DataValue::Str(account.email.clone().into()))
            ]
        };
        let params = params.into_iter().map(|(a, b)| (String::from(a), b)).collect::<Vec<_>>();
        let result = self.db.run_script("
            ?[account_type, account_active, account_key, account_email ] <- [[ $type, $active, $key, $email ]]
            :replace settings {
                account_type: String,
                account_active: Bool,
                account_key: String,
                account_email: String
            }
        ", params.into_iter().collect()).unwrap();
    }

    pub fn get_settings(&self) -> Settings {
        let result = self.db.run_script("
            ?[type, active, key, email ] :=
                *settings { account_type: type, account_active: active, account_key: key, account_email: email }
        ", BTreeMap::new());

        match result {
            Ok(result) => {
                let row = result.rows.into_iter().next().unwrap_or_default();

                use DataValue::*;
                match &row[..] {
                    [ Str(acc_type), Bool(active), Str(key), Str(email) ] => {
                        match &**acc_type {
                            "direct" => Settings {
                                account: Some(Account::Direct(AccountDirect(key.to_string())))
                            },
                            "aerome" => Settings {
                                account: Some(Account::Aerome(AccountAerome {
                                    active: *active,
                                    key: key.to_string(),
                                    email: email.to_string()
                                }))
                            },
                            _ => Settings::default()
                        }
                    },
                    _ => Settings::default()
                }
            },
            Err(_) => Settings::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn settings_direct_account() {
        let store = Store::new();
        let direct = Account::Direct(AccountDirect("foobar".into()));

        store.set_account(&direct);

        assert_eq!(store.get_settings(), Settings {
            account: Some(direct)
        });
    }

    #[test]
    #[serial]
    fn settings_aerome_account() {
        let store = Store::new();
        let aerome = Account::Aerome(AccountAerome {
            key: "boobar".into(),
            active: false
        });

        store.set_account(&aerome);

        assert_eq!(store.get_settings(), Settings {
            account: Some(aerome)
        });
    }
}
