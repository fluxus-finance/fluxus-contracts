use crate::*;

/// Splits farm_id_str
/// Returns seed_id, token_id, farm_id
/// (exchange@pool_id, :pool_id, farm_id) => ref-finance@10, :10, 0
// TODO: can it be a &str?
pub fn get_ids_from_farm(farm_id_str: String) -> (String, String, String) {
    let ids: Vec<&str> = farm_id_str.split('#').collect();
    let token_id: Vec<&str> = ids[0].split('@').collect();

    let token_id_wrapped = format!(":{}", token_id[1]);

    (ids[0].to_owned(), token_id_wrapped, ids[1].to_owned())
}

pub fn get_predecessor_and_current_account() -> (AccountId, AccountId) {
    (env::predecessor_account_id(), env::current_account_id())
}

pub fn unwrap_token_id(token_id: &str) -> String {
    let mut chars = token_id.chars();
    chars.next();

    chars.collect()
}

/// wrap token_id into correct format in MFT standard
pub fn wrap_mft_token_id(pool_id: &str) -> String {
    format!(":{}", pool_id)
}

/// Assert that the farm_id_str is valid, meaning that the farm is Running
pub fn assert_strategy_not_cleared(state: AutoCompounderState) {
    match state {
        AutoCompounderState::Running => (),
        AutoCompounderState::Ended => (),
        _ => env::panic_str("E51: strategy ended"),
    };
}
