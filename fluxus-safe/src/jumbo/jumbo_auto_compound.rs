use crate::*;

// const MIN_SLIPPAGE_ALLOWED: u128 = 1;

#[near_bindgen]
impl Contract {
    /// Check if farm still have rewards to distribute (status == Running)
    /// # Parameters example: 
    ///   farm_id_str: exchange@pool_id#farm_id
    #[private]
    pub fn callback_jumbo_list_farms_by_seed(
        &mut self,
        #[callback_result] farms_result: Result<Vec<FarmInfo>, PromiseError>,
        farm_id_str: String,
    ) -> PromiseOrValue<String> {
        assert!(farms_result.is_ok(), "{}", ERR01_LIST_FARMS_FAILED);

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let farms = farms_result.unwrap();

        // Try to unclaim before change to Ended
        for farm in farms.iter() {
            if farm.farm_id == farm_id && farm.farm_status != *"Running" {
                let compounder = self.get_strat_mut(&seed_id).get_jumbo_mut();

                for strat_farm in compounder.farms.iter_mut() {
                    if strat_farm.id == farm_id {
                        strat_farm.state = JumboAutoCompounderState::Ended;
                    }
                }
            }
        }

        let compounder = self.get_strat(&seed_id).get_jumbo();

        PromiseOrValue::Promise(
            ext_jumbo_farming::get_unclaimed_reward(
                env::current_account_id(),
                farm_id_str.clone(),
                compounder.farm_contract_id,
                0,
                Gas(3_000_000_000_000),
            )
            .then(
                callback_jumbo_exchange::callback_jumbo_post_get_unclaimed_reward(
                    farm_id_str,
                    env::current_account_id(),
                    0,
                    Gas(70_000_000_000_000),
                ),
            ),
        )
    }

    /// Check the reward amount earned and  claim reward by farm.
    /// # Parameters example: 
    ///   farm_id_str: exchange@pool_id#farm_id
    #[private]
    pub fn callback_jumbo_post_get_unclaimed_reward(
        &mut self,
        #[callback_result] reward_amount_result: Result<U128, PromiseError>,
        farm_id_str: String,
    ) -> PromiseOrValue<u128> {
        assert!(reward_amount_result.is_ok(), "{}", ERR02_GET_REWARD_FAILED);

        let reward_amount = reward_amount_result.unwrap();

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let strat = self.get_strat_mut(&seed_id);

        let compounder = strat.get_jumbo_mut();

        let farm_info = compounder.get_mut_jumbo_farm_info(farm_id);

        if reward_amount.0 == 0u128 {
            // if farm is ended, there is no more actions to do
            if farm_info.state == JumboAutoCompounderState::Ended {
                farm_info.state = JumboAutoCompounderState::Cleared;
                return PromiseOrValue::Value(0u128);
            } else {
                panic!("{}", ERR03_CLAIM_FAILED)
            }
        }

        // store the amount of reward earned
        farm_info.last_reward_amount = reward_amount.0;

        PromiseOrValue::Promise(
            ext_jumbo_farming::claim_reward_by_farm(
                farm_id_str.clone(),
                compounder.farm_contract_id.clone(),
                0,
                Gas(40_000_000_000_000),
            )
            .then(callback_jumbo_exchange::callback_jumbo_post_claim_reward(
                farm_id_str,
                env::current_account_id(),
                0,
                Gas(10_000_000_000_000),
            )),
        )
    }

    /// Make sure that the reward was claimed and update the compounder cycle.
    /// # Parameters example: 
    ///   farm_id_str: exchange@pool_id#farm_id
    #[private]
    pub fn callback_jumbo_post_claim_reward(
        &mut self,
        #[callback_result] claim_reward_result: Result<(), PromiseError>,
        farm_id_str: String,
    ) {
        assert!(claim_reward_result.is_ok(), "{}", ERR03_CLAIM_FAILED);

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str);

        let compounder = self.get_strat_mut(&seed_id).get_jumbo_mut();
        let farm_info = compounder.get_mut_jumbo_farm_info(farm_id);
        farm_info.next_cycle();
    }

    /// Make sure that the withdraw was ok, store the fees correctly and transfer the amount to the exchange contract.
    /// # Parameters example: 
    ///   farm_id_str: exchange@pool_id#farm_id
    #[private]
    pub fn callback_jumbo_post_withdraw(
        &mut self,
        #[callback_result] withdraw_result: Result<(), PromiseError>,
        farm_id_str: String,
    ) -> PromiseOrValue<U128> {
        assert!(
            withdraw_result.is_ok(),
            "{}",
            ERR04_WITHDRAW_FROM_FARM_FAILED
        );

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let data_mut = self.data_mut();

        let strat = data_mut
            .strategies
            .get_mut(&seed_id)
            .expect(ERR42_TOKEN_NOT_REG);

        let compounder = strat.get_jumbo_mut();

        let last_reward_amount = compounder
            .get_mut_jumbo_farm_info(farm_id.clone())
            .last_reward_amount;

        let (remaining_amount, protocol_amount, sentry_amount, strat_creator_amount) =
            compounder.compute_fees(last_reward_amount);

        // storing the amount earned by the strat creator
        compounder.admin_fees.strat_creator.current_amount += strat_creator_amount;

        // store sentry amount under contract account id to be used in the last step
        compounder
            .admin_fees
            .sentries
            .insert(env::current_account_id(), sentry_amount);

        // increase protocol amount to cover the case that the last transfer failed
        data_mut.treasury.current_amount += protocol_amount;

        // remaining amount to reinvest
        compounder
            .get_mut_jumbo_farm_info(farm_id.clone())
            .last_reward_amount = remaining_amount;

        // amount sent to ref, both remaining value and treasury
        let amount = remaining_amount + protocol_amount;

        PromiseOrValue::Promise(
            ext_reward_token::ft_transfer_call(
                compounder.exchange_contract_id.clone(),
                U128(amount), //Amount after withdraw the rewards
                "".to_string(),
                compounder
                    .get_mut_jumbo_farm_info(farm_id)
                    .reward_token
                    .clone(),
                1,
                Gas(140_000_000_000_000),
            )
            .then(callback_jumbo_exchange::callback_jumbo_post_ft_transfer(
                farm_id_str,
                env::current_account_id(),
                0,
                Gas(15_000_000_000_000),
            )),
        )
    }

    /// Make sure that the transfer succeeded and update the compounder cycle.
    /// # Parameters example: 
    ///   farm_id_str: exchange@pool_id#farm_id
    #[private]
    pub fn callback_jumbo_post_ft_transfer(
        &mut self,
        #[callback_result] exchange_transfer_result: Result<U128, PromiseError>,
        farm_id_str: String,
    ) {
        if exchange_transfer_result.is_err() {
            log!(ERR07_TRANSFER_TO_EXCHANGE);
            return;
        }

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str);

        let data_mut = self.data_mut();
        let strat = data_mut
            .strategies
            .get_mut(&seed_id)
            .expect(ERR42_TOKEN_NOT_REG);

        let compounder = strat.get_jumbo_mut();
        let farm_info_mut = compounder.get_mut_jumbo_farm_info(farm_id);
        farm_info_mut.next_cycle();
    }

    /// Callback to verify that transfer to treasure succeeded
    #[private]
    pub fn callback_jumbo_post_treasury_mft_transfer(
        &mut self,
        #[callback_result] ft_transfer_result: Result<(), PromiseError>,
    ) {
        let data_mut = self.data_mut();

        // in the case where the transfer failed, the next cycle will send it plus the new amount earned
        if ft_transfer_result.is_err() {
            log!(ERR08_TRANSFER_TO_TREASURE);
            return;
        }

        let amount: u128 = data_mut.treasury.current_amount;

        // reset treasury amount earned since tx was successful
        data_mut.treasury.current_amount = 0;

        log!("Transfer {} to treasure succeeded", amount)
    }

    /// Make sure that the transfer to the creator succeeded.
    /// # Parameters example: 
    ///   seed_id: exchange@pool_id
    #[private]
    pub fn callback_jumbo_post_creator_ft_transfer(
        &mut self,
        #[callback_result] strat_creator_transfer_result: Result<(), PromiseError>,
        seed_id: String,
    ) {
        if strat_creator_transfer_result.is_err() {
            log!(ERR09_TRANSFER_TO_CREATOR);
            return;
        }

        let strat = self.get_strat_mut(&seed_id);
        let compounder = strat.get_jumbo_mut();

        compounder.admin_fees.strat_creator.current_amount = 0;
        log!("Transfer fees to the creator of the strategy succeeded");
    }

    /// Make sure that the swap is possible and call it.
    /// # Parameters example: 
    ///   farm_id_str: exchange@pool_id#farm_id
    ///   amount_token_1: U128(1000000)
    #[private]
    pub fn callback_jumbo_get_token1_return(
        &mut self,
        #[callback_result] min_amount_out: Result<U128, PromiseError>,
        farm_id_str: String,
        amount_token_1: U128,
    ) -> PromiseOrValue<u128> {
        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        if min_amount_out.is_err() {
            log!(ERR10_SWAP_TOKEN);
            let compounder = self.get_strat_mut(&seed_id).get_jumbo_mut();
            let farm_info_mut = compounder.get_mut_jumbo_farm_info(farm_id);
            farm_info_mut.increase_slippage();

            return PromiseOrValue::Value(0u128);
        }

        let min_out = min_amount_out.unwrap();

        assert!(min_out.0 > 0, "{}", ERR10_SWAP_TOKEN);

        log!("Min out for token 1: {}", min_out.0);

        let compounder = self.get_strat(&seed_id).get_jumbo();
        let farm_info = compounder.get_jumbo_farm_info(&farm_id);

        PromiseOrValue::Promise(
            // 100 TGAS
            self.jumbo_call_swap(
                compounder.exchange_contract_id,
                farm_info.pool_id_token1_reward,
                farm_info.reward_token,
                compounder.token1_address,
                Some(amount_token_1),
                min_out,
            )
            .then(callback_jumbo_exchange::callback_jumbo_post_first_swap(
                farm_id_str,
                amount_token_1,
                min_out,
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            )),
        )
    }

    /// Make sure that the swap succeeded and update compounder cycles.
    /// # Parameters example: 
    ///   farm_id_str: exchange@pool_id#farm_id
    ///   amount_in: U128(1000000)
    ///   min_amount_out: U128(1000000)
    #[private]
    pub fn callback_jumbo_post_first_swap(
        &mut self,
        #[callback_result] swap_result: Result<(), PromiseError>,
        farm_id_str: String,
        amount_in: U128,
        min_amount_out: U128,
    ) -> U128 {
        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str);
        let compounder_mut = self.get_strat_mut(&seed_id).get_jumbo_mut();
        let farm_info_mut = compounder_mut.get_mut_jumbo_farm_info(farm_id);

        // Do not panic if err == true, otherwise the slippage update will not be applied
        if swap_result.is_err() {
            farm_info_mut.increase_slippage();
            log!(ERR10_SWAP_TOKEN);

            return U128(0u128);
        }

        farm_info_mut.available_balance[0] = min_amount_out.0;

        // First swap succeeded, thus decrement the last reward_amount
        farm_info_mut.last_reward_amount -= amount_in.0;

        farm_info_mut.next_cycle();

        min_amount_out
    }

    /// Make sure that the swap is possible and call it.
    /// # Parameters example: 
    ///   farm_id_str: exchange@pool_id#farm_id
    ///   amount_token_2: exchange@pool_id#farm_id
    #[private]
    pub fn callback_jumbo_get_token2_return(
        &mut self,
        #[callback_result] min_amount_out: Result<U128, PromiseError>,
        farm_id_str: String,
        amount_token_2: U128,
    ) -> PromiseOrValue<u128> {
        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        if min_amount_out.is_err() {
            log!(ERR10_SWAP_TOKEN);
            let compounder = self.get_strat_mut(&seed_id).get_jumbo_mut();
            let farm_info_mut = compounder.get_mut_jumbo_farm_info(farm_id);
            farm_info_mut.increase_slippage();

            return PromiseOrValue::Value(0u128);
        }

        let min_out = min_amount_out.unwrap();

        assert!(min_out.0 > 0, "{}", ERR05_COULD_NOT_GET_RETURN_FOR_TOKEN);

        log!("Min out for token 2: {}", min_out.0);

        let compounder = self.get_strat(&seed_id).get_jumbo();
        let farm_info = compounder.get_jumbo_farm_info(&farm_id);

        PromiseOrValue::Promise(
            // 100 TGAS
            self.jumbo_call_swap(
                compounder.exchange_contract_id,
                farm_info.pool_id_token2_reward,
                farm_info.reward_token,
                compounder.token2_address,
                Some(amount_token_2),
                min_out,
            )
            .then(callback_jumbo_exchange::callback_jumbo_post_second_swap(
                farm_id_str,
                amount_token_2,
                min_out,
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            )),
        )
    }

    /// Make sure that the swap succeeded and update compounder cycles.
    /// # Parameters example: 
    ///   farm_id_str: exchange@pool_id#farm_id
    ///   amount_in: U128(1000000)
    ///   min_amount_out: U128(1000000)
    #[private]
    pub fn callback_jumbo_post_second_swap(
        &mut self,
        #[callback_result] swap_result: Result<(), PromiseError>,
        farm_id_str: String,
        amount_in: U128,
        min_amount_out: U128,
    ) -> U128 {
        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str);
        let compounder_mut = self.get_strat_mut(&seed_id).get_jumbo_mut();
        let farm_info_mut = compounder_mut.get_mut_jumbo_farm_info(farm_id);

        // Do not panic if err == true, otherwise the slippage update will not be applied
        if swap_result.is_err() {
            farm_info_mut.increase_slippage();
            log!(ERR10_SWAP_TOKEN);

            return U128(0u128);
        }

        farm_info_mut.available_balance[1] = min_amount_out.0;

        // First swap succeeded, thus decrement the last reward_amount
        farm_info_mut.last_reward_amount -= amount_in.0;

        // after both swaps succeeded, it's ready to stake
        farm_info_mut.next_cycle();

        min_amount_out
    }

    /// Make sure that the caller is register, has balance and then transfer to sentry.
    /// # Parameters example: 
    ///   farm_id_str: exchange@pool_id#farm_id
    ///   sentry_acc_id: sentry.testnet
    ///   reward_token: reward.testnet
    #[private]
    pub fn callback_jumbo_post_sentry(
        &mut self,
        #[callback_result] result: Result<Option<StorageBalance>, PromiseError>,
        farm_id_str: String,
        sentry_acc_id: AccountId,
        reward_token: AccountId,
    ) -> Promise {
        // TODO: propagate error
        match result {
            Ok(balance_op) => match balance_op {
                Some(balance) => assert!(balance.total.0 > 1),
                _ => {
                    // let msg = ("ERR: callback post Sentry no balance {:#?} ",balance_op);
                    let msg = format!(
                        "{}{:#?}",
                        ERR11_NOT_ENOUGH_BALANCE,
                        balance_op
                            .unwrap_or(StorageBalance {
                                total: U128(0),
                                available: U128(0)
                            })
                            .total
                    );
                    env::panic_str(msg.as_str());
                }
            },
            Err(_) => env::panic_str(ERR12_CALLER_NOT_REGISTER),
        }

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());
        let compounder = self.get_strat_mut(&seed_id).get_jumbo_mut();

        // reset default sentry address and get last earned amount
        let amount = compounder
            .admin_fees
            .sentries
            .remove(&env::current_account_id())
            .unwrap();

        let farm_info_mut = compounder.get_mut_jumbo_farm_info(farm_id);

        // if farm is ended, there is no more actions to do
        if farm_info_mut.state == JumboAutoCompounderState::Ended {
            farm_info_mut.state = JumboAutoCompounderState::Cleared;

            log!("There farm {} ended. Strategy is now Cleared.", farm_id_str);
        }

        if amount > 0 {
            ext_reward_token::ft_transfer(
                sentry_acc_id.clone(),
                U128(amount),
                Some("".to_string()),
                reward_token,
                1,
                Gas(20_000_000_000_000),
            )
            .then(
                callback_jumbo_exchange::callback_jumbo_post_sentry_mft_transfer(
                    farm_id_str,
                    sentry_acc_id,
                    amount,
                    env::current_account_id(),
                    0,
                    Gas(240_000_000_000_000),
                ),
            )
        } else {
            log!("Sentry earned 0 reward");
            self.jumbo_harvest_add_liquidity(farm_id_str)
        }
    }

    /// Make sure that the transfer succeeded and call add_liquidity.
    /// # Parameters example: 
    ///   farm_id_str: exchange@pool_id#farm_id
    ///   sentry_id: sentry.testnet
    ///   amount_earned: 10000
    #[private]
    pub fn callback_jumbo_post_sentry_mft_transfer(
        &mut self,
        #[callback_result] ft_transfer_result: Result<(), PromiseError>,
        farm_id_str: String,
        sentry_id: AccountId,
        amount_earned: u128,
    ) -> PromiseOrValue<u64> {
        let (seed_id, token_id, _) = get_ids_from_farm(farm_id_str.to_string());

        // in the case where the transfer failed, the next cycle will send it plus the new amount earned
        if ft_transfer_result.is_err() {
            log!(ERR13_TRANSFER_TO_SENTRY);

            let compounder = self.get_strat_mut(&seed_id).get_jumbo_mut();

            // store amount earned by sentry to be redeemed
            compounder
                .admin_fees
                .sentries
                .insert(sentry_id, amount_earned);
        }

        PromiseOrValue::Promise(self.jumbo_harvest_add_liquidity(farm_id_str))
    }

    /// Call jumbo's add_liquidity.
    /// # Parameters example: 
    ///   farm_id_str: exchange@pool_id#farm_id
    #[private]
    pub fn jumbo_harvest_add_liquidity(&mut self, farm_id_str: String) -> Promise {
        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let compounder = self.get_strat(&seed_id).get_jumbo();
        let farm_info = compounder.get_jumbo_farm_info(&farm_id);

        let pool_id: u64 = compounder.pool_id;

        let token1_amount = farm_info.available_balance[0];
        let token2_amount = farm_info.available_balance[1];

        ext_jumbo_exchange::add_liquidity(
            pool_id,
            vec![U128(token1_amount), U128(token2_amount)],
            None,
            compounder.exchange_contract_id,
            970000000000000000000,
            Gas(80_000_000_000_000),
        )
        .then(callback_jumbo_exchange::callback_jumbo_post_add_liquidity(
            farm_id_str,
            env::current_account_id(),
            0,
            Gas(130_000_000_000_000),
        ))
    }

    /// Make sure that the liquidity was added and get the new amount of pool shares.
    /// # Parameters example: 
    ///   farm_id_str: exchange@pool_id#farm_id
    #[private]
    pub fn callback_jumbo_post_add_liquidity(
        &mut self,
        #[callback_result] shares_result: Result<(), PromiseError>,
        farm_id_str: String,
    ) -> Promise {
        assert!(shares_result.is_ok(), "{}", ERR14_ADD_LIQUIDITY);

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let compounder_mut = self.get_strat_mut(&seed_id).get_jumbo_mut();
        let farm_info_mut = compounder_mut.get_mut_jumbo_farm_info(farm_id);

        // ensure that in the next run we won't have a balance unless previous steps succeeds
        farm_info_mut.available_balance[0] = 0u128;
        farm_info_mut.available_balance[1] = 0u128;

        ext_jumbo_exchange::get_pool_shares(
            compounder_mut.pool_id,
            env::current_account_id(),
            compounder_mut.exchange_contract_id.clone(),
            0,
            Gas(10_000_000_000_000),
        )
        // Update user balance and stake
        .then(
            callback_jumbo_exchange::callback_jumbo_post_get_pool_shares(
                farm_id_str,
                env::current_account_id(),
                0,
                Gas(100_000_000_000_000),
            ),
        )
    }

    /// Update the amount of seed and the compounder cycle.
    /// # Parameters example: 
    ///   farm_id_str: exchange@pool_id#farm_id
    ///   shares_on_exchange: 100000
    #[private]
    pub fn update_shares_and_forward_cycle(
        &mut self,
        farm_id_str: String,
        shares_on_exchange: u128,
    ) -> u128 {
        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str);

        let total_seed = self.seed_total_amount(&seed_id);

        let compounder_mut = self.get_strat_mut(&seed_id).get_jumbo_mut();
        let farm_info_mut = compounder_mut.get_mut_jumbo_farm_info(farm_id);
        log!("seed: {}", seed_id);

        // this can happen when there is more than one strat for the same pool
        // which will move the shares on the exchange to the farm
        // leaving the value on exchange lower than the shares stored in the contract
        if farm_info_mut.current_shares_to_stake > shares_on_exchange {
            let new_seed_amount = shares_on_exchange + total_seed;

            log!(
            "Inside update shares\ntotal seed: {} accumulated: {} current: {}\nnew seed_amount: {}",
            total_seed,
            shares_on_exchange,
            farm_info_mut.current_shares_to_stake,
            new_seed_amount
            );

            farm_info_mut.current_shares_to_stake = shares_on_exchange;

            farm_info_mut.next_cycle();

            return new_seed_amount;
        }

        // update seed total amount
        let curr_shares = shares_on_exchange - farm_info_mut.current_shares_to_stake;

        let new_seed_amount = curr_shares + total_seed;

        log!(
            "Inside update shares\ntotal seed: {} accumulated: {} current: {}\nnew seed_amount: {}",
            total_seed,
            shares_on_exchange,
            farm_info_mut.current_shares_to_stake,
            new_seed_amount
        );

        farm_info_mut.current_shares_to_stake = shares_on_exchange;

        farm_info_mut.next_cycle();

        new_seed_amount
    }

    /// Receives shares from auto-compound and stake it.
    /// # Parameters example: 
    ///   farm_id_str: exchange@pool_id#farm_id
    #[private]
    pub fn callback_jumbo_post_get_pool_shares(
        &mut self,
        #[callback_result] total_shares_result: Result<U128, PromiseError>,
        farm_id_str: String,
    ) -> PromiseOrValue<u64> {
        assert!(total_shares_result.is_ok(), "{}", ERR17_GET_POOL_SHARES);

        let shares_on_exchange = total_shares_result.unwrap().0;

        log!("accumulated shares: {}", shares_on_exchange);

        let (seed_id, token_id, _) = get_ids_from_farm(farm_id_str.clone());

        let new_seed_amount =
            self.update_shares_and_forward_cycle(farm_id_str.clone(), shares_on_exchange);

        self.data_mut()
            .seed_id_amount
            .insert(&seed_id, &new_seed_amount);

        let compounder = self.get_strat(&seed_id).get_jumbo();

        // Prevents failing on stake if below minimum deposit
        if shares_on_exchange < compounder.seed_min_deposit.into() {
            log!(
                "The current number of shares {} is below minimum deposit",
                shares_on_exchange
            );
            return PromiseOrValue::Value(0u64);
        }

        // return PromiseOrValue::Value(0u64);

        PromiseOrValue::Promise(
            self.call_stake(
                compounder.exchange_contract_id.clone(),
                compounder.farm_contract_id,
                token_id,
                U128(shares_on_exchange),
                "".to_string(),
            )
            .then(
                callback_jumbo_exchange::callback_jumbo_post_stake_from_harvest(
                    farm_id_str,
                    env::current_account_id(),
                    0,
                    Gas(10_000_000_000_000),
                ),
            ),
        )
    }

    /// Make sure that the stake succeeded.
    /// # Parameters example: 
    ///   farm_id_str: exchange@pool_id#farm_id
    #[private]
    pub fn callback_jumbo_post_stake_from_harvest(
        &mut self,
        #[callback_result] stake_result: Result<U128, PromiseError>,
        farm_id_str: String,
    ) {
        assert!(stake_result.is_ok());

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str);

        let compounder_mut = self.get_strat_mut(&seed_id).get_jumbo_mut();
        let farm_info_mut = compounder_mut.get_mut_jumbo_farm_info(farm_id);

        // reset shares after staking
        farm_info_mut.current_shares_to_stake = 0;
    }
}
