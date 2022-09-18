
pub const ERR21_TOKEN_NOT_REG: &str = "E21: token not registered";
pub const ERR33_TRANSFER_TO_SELF: &str = "E33: transfer to self";

//Jumbo
pub const ERR01_LIST_FARMS_FAILED: &str = "E01: It was not possible to get the jumbo's list of farms.";
pub const ERR02_GET_REWARD_FAILED: &str = "E02: It was not possible to get the contract`s reward.";
pub const ERR03_CLAIM_FAILED: &str = "E03: Fail trying to claim the rewards.";
pub const ERR04_WITHDRAW_FROM_FARM_FAILED: &str = "E04: Fail trying to withdraw the rewards.";
pub const ERR05_COULD_NOT_GET_RETURN_FOR_TOKEN: &str = "E05: Fail trying to get the amount_out.";
pub const ERR06_ZERO_REWARDS_EARNED: &str = "E06: Contract earned zero rewards";
pub const ERR07_TRANSFER_TO_EXCHANGE: &str = "E07: Fail transferring to the exchange.";
pub const ERR08_TRANSFER_TO_TREASURE: &str = "E08: Fail trying to transfer to treasure.";
pub const ERR09_TRANSFER_TO_CREATOR: &str = "E09: Fail trying to transfer to the strategy creator.";
pub const ERR10_SWAP_TOKEN: &str = "E10: fail trying to swap tokens.";
pub const ERR11_NOT_ENOUGH_BALANCE: &str = "E11: Not enough balance to the storage.";
pub const ERR12_CALLER_NOT_REGISTER: &str = "E12: The caller in not register in the reward_token contract.";
pub const ERR13_TRANSFER_TO_SENTRY: &str = "E13: Transferring to the sentry contract.";
pub const ERR14_ADD_LIQUIDITY: &str = "E14: fail adding liquidity.";
pub const ERR15_TOTAL_SHARES: &str = "E15: could not get total seed.";
pub const ERR16_STAKE_FAILED: &str = "E16: It was not possible to stake.";
pub const ERR17_GET_POOL_SHARES: &str = "E17: It was not possible to get the pool shares ";
pub const ERR18_JUMBO_WITHDRAW: &str = "E18: It was not possible to withdraw the shares.";

//Pembrock
pub const ERR19_CLAIMED_ZERO_AMOUNT: &str = "E19: Claimed zero rewards.";
pub const ERR20_SEED_ID_DOES_NOT_EXIST: &str = "E20: The seed_id does not have strategies.";

//General
pub const ERR21_CAN_NOT_DEPOSIT_INTO_LOST_FOUND: &str = "E21: non-whitelisted token can NOT deposit into lost-found.";
pub const ERR22_NO_AVAILABLE_STORAGE_TO_WITHDRAW: &str = "E22: There is no available storage to withdraw.";
pub const ERR23_NOT_ENOUGH_AVAILABLE_STORAGE_TO_WITHDRAW: &str = "E23: There is not enough available storage to withdraw.";
pub const ERR24_VERSIONED_STRATEGY_ALREADY_EXIST: &str = "E24: A versioned strategy with this parameters already exist.";
pub const ERR25_FARM_ID_ALREADY_EXIST_FOR_SEED: &str = "E25: This farm_id already has a correspondent farm.";
pub const ERR26_FEE_NOT_VALID: &str = "E26: The fee amount is not valid.";
pub const ERR27_FEE_TOO_HIGH: &str = "E26: The fee amount is too high.";
pub const ERR28_CONTRACT_ALREADY_INITIALIZED: &str = "E28: The contract is already initialized.";
pub const ERR29_CONTRACT_PAUSED: &str = "E29: The contract is paused.";
pub const ERR30_NO_RUNNING_STRATEGIES: &str = "E30: There is no running strategies for this pool.";
pub const ERR31_FAIL_GETTING_TOKEN_ID: &str = "E31: It was not possible to get the token_id.";
pub const ERR32_SEED_DOES_NOT_EXIST: &str = "E32: The seed_id does not exist.";
pub const ERR33_FFT_SHARE_DOES_NOT_EXIST: &str = "E33: The fft_share does not exist.";
pub const ERR34_NOT_ALLOWED: &str = "E34: The caller is not allowed to do this.";
pub const ERR35_ALREADY_ALLOWED: &str = "E35: The caller is already allowed.";
pub const ERR36_ACCOUNT_DOES_NOT_EXIST: &str = "E36: The account does not exist.";
pub const ERR37_PROMISE_FAILED: &str = "E37: The promise has failed";
pub const ERR38_LESS_THAN_MIN_STORAGE: &str = "E38: The amount deposited is less than the mim deposit allowed.";
pub const ERR39_ACCOUNT_ALREADY_REGISTER: &str = "E39: The account is already register in the contract.";
pub const ERR40_STORAGE_UNREGISTER_TOKENS_NOT_EMPTY: &str = "E40: The token amount is not empty. Cannot unregister it.";
pub const ERR41_STRATEGY_ENDED: &str = "E51: The strategy is Ended.";


