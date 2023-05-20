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

use dirs;
use cozo::{self,Db,SqliteStorage,DataValue};
use crate::constants::APP_NAME;
use crate::models::{Action,Account,AccountDirect,AccountAerome,Settings,Suggestions};
use std::{fs,path::{PathBuf,Path},collections::BTreeMap};

#[derive(Clone)]
pub struct Store {
    db: Db<SqliteStorage>
}

impl Store {
    pub fn new() -> Self {
        let data_dir = dirs::data_dir().unwrap().join(APP_NAME);
        let db_file = if cfg!(test) { "store_test.db" } else { "store.db" };
        let store_file = data_dir.join(db_file);

        if !data_dir.exists() {
            fs::create_dir_all(data_dir).unwrap();
        }

        let store_exists = store_file.exists();
        let db = cozo::new_cozo_sqlite(store_file).expect("Could not create database store");
        let _ = db.run_script(r#"
            :create actions {
                code: String,
                path: String,
                question: String,
                description: String =>
                inserted: Validity default 'ASSERT'
            }
        "#, Default::default());

        Store { db }
    }

    pub fn add_suggestion(&self, path: &Path, action: &Action) {
        let params: BTreeMap<String, DataValue> = vec![
            (String::from("code"), DataValue::Str(action.code.clone().into())),
            (String::from("path"), DataValue::Str(path.to_str().unwrap().into())),
            (String::from("question"), DataValue::Str(action.question.clone().into())),
            (String::from("description"), DataValue::Str(action.description
                .clone()
                .unwrap_or_else(|| "".into())
                .into()
            ))
        ].into_iter().collect();

        let result = self.db.run_script("
            ?[ code, path, question, description ] <- [
                [ $code, $path, $question, $description ]
            ]
            :put actions {
                code,
                path,
                question,
                description
            }
        ", params).unwrap();
    }

    pub fn get_suggestions(&self, path: &Path) -> Suggestions {
        let params: BTreeMap<String, DataValue> = vec![
            (String::from("path"), DataValue::Str(path.to_str().unwrap().into())),
        ].into_iter().collect();

        let result = self.db.run_script("
            ?[ code, description, path, question, inserted ] :=
                *actions { code, description, path, question, inserted },
                path == $path

            :sort inserted
        ", params).unwrap();

        let actions = result.rows.into_iter()
            .filter_map(|row| {
                use DataValue::*;
                match &row[..] {
                    [ Str(code), Str(description), Str(path), Str(question), _ ] => Some(Action {
                        code: (&**code).to_owned(),
                        question: (&**question).to_owned(),
                        description: Some((&**description).to_owned())
                    }),
                    _ => None
                }
            })
            .collect();

        Suggestions {
            purpose: String::new(),
            actions
        }
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
    fn settings_action() {
        let store = Store::new();
        let action = Action {
            code: "some code".into(),
            question: "How's it goin?".into(),
            description: Some("humho".into())
        };
        let other = Action {
            code: "some code".into(),
            question: "How's it goin?".into(),
            description: None
        };

        store.add_suggestion(&PathBuf::from("/foo/bar"), &action);
        store.add_suggestion(&PathBuf::from("/foo/boo"), &other);

        assert_eq!(1, store.get_suggestions(&PathBuf::from("/foo/bar")).actions.len());
    }

    #[test]
    #[serial]
    fn settings_direct_account() {
        let store = Store::new();
        let direct = Account::Direct(AccountDirect("foobar".into()));

        store.set_account(&Some(direct.clone()));

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
            email: "humho@gmail.com".into(),
            active: false
        });

        store.set_account(&Some(aerome.clone()));

        assert_eq!(store.get_settings(), Settings {
            account: Some(aerome)
        });
    }
}
