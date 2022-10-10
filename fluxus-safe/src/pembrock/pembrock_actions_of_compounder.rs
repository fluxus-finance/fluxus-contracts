use crate::*;

#[near_bindgen]
impl Contract {
    /// Make sure that the stake succeeded and mint the correct amount to the user.
    /// # Parameters example:
    ///   seed_id: exchange@pool_id
    ///   account_id: account.testnet
    ///   shares: 10000000
    #[private]
    pub fn callback_pembrock_stake_result(
        &mut self,
        #[callback_result] transfer_result: Result<U128, PromiseError>,
        seed_id: String,
        account_id: AccountId,
        shares: u128,
    ) -> String {
        if let Ok(amount) = transfer_result {
            assert_ne!(amount.0, 0, "{}", ERR16_STAKE_FAILED);
        }

        //Total fft_share
        let total_fft = self.total_supply_by_pool_id(seed_id.clone());

        let fft_share_id = self.get_fft_share_id_from_seed(seed_id.clone());

        let data = self.data_mut();

        //Total seed_id
        let total_seed = data.seed_id_amount.get(&seed_id).unwrap_or_default();

        self.data_mut()
            .seed_id_amount
            .insert(&seed_id, &(total_seed + shares));

        let fft_share_amount = if total_fft == 0 {
            shares
        } else {
            (U256::from(shares) * U256::from(total_fft) / U256::from(total_seed)).as_u128()
        };

        log!(
            "{} {} will be minted for {}",
            fft_share_amount,
            fft_share_id,
            account_id.to_string()
        );
        self.mft_mint(fft_share_id, fft_share_amount, account_id.to_string());

        format!(
            "The {} added {} to {}",
            account_id, fft_share_amount, seed_id
        )
    }

    /// Make sure that the claim succeeded and transfer some amount to the strategy creator.
    /// # Parameters example:
    ///   strat_name: pembrock@token_name
    #[private]
    pub fn callback_pembrock_rewards(
        &mut self,
        #[callback_result] claim_result: Result<U128, PromiseError>,
        strat_name: String,
    ) -> PromiseOrValue<u128> {
        assert!(claim_result.is_ok(), "{}", ERR03_CLAIM_FAILED);

        let claimed = claim_result.unwrap().0;

        assert!(claimed > 0, "{}", ERR19_CLAIMED_ZERO_AMOUNT);

        let data_mut = self.data_mut();

        let strat = data_mut
            .strategies
            .get_mut(&strat_name)
            .expect(ERR42_TOKEN_NOT_REG);

        let compounder = strat.get_pemb_mut();

        let (remaining_amount, protocol_amount, sentry_amount, strat_creator_amount) =
            compounder.compute_fees(claimed);

        compounder.last_reward_amount += remaining_amount;

        compounder.strat_creator_fee_amount += strat_creator_amount;

        // store sentry amount under contract account id to be used in the last step
        compounder
            .admin_fees
            .sentries
            .insert(env::current_account_id(), sentry_amount);

        compounder.treasury.current_amount += protocol_amount;

        compounder.next_cycle();
        log!(
            "last_reward_amount for {}: {}",
            strat_name,
            compounder.last_reward_amount
        );

        if protocol_amount > 0 {
            ext_reward_token::ft_transfer(
                compounder.admin_fees.strat_creator_account_id.clone(),
                U128(strat_creator_amount),
                Some("".to_string()),
                compounder.reward_token.clone(),
                1,
                Gas(50_000_000_000_000),
            )
            .then(callback_pembrock::callback_pembrock_post_treasury_transfer(
                strat_name.clone(),
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            ));
        }

        if strat_creator_amount > 0 {
            ext_reward_token::ft_transfer(
                compounder.admin_fees.strat_creator_account_id.clone(),
                U128(strat_creator_amount),
                Some("".to_string()),
                compounder.reward_token.clone(),
                1,
                Gas(50_000_000_000_000),
            )
            .then(
                callback_pembrock::callback_pembrock_post_creator_ft_transfer(
                    strat_name,
                    env::current_account_id(),
                    0,
                    Gas(20_000_000_000_000),
                ),
            );
        }

        PromiseOrValue::Value(0u128)
    }

    /// Ensure that the transfer to the treasury succeeded.
    #[private]
    pub fn callback_pembrock_post_treasury_transfer(
        &mut self,
        #[callback_result] transfer_result: Result<(), PromiseError>,
        strat_name: String,
    ) {
        match transfer_result {
            Ok(_) => {
                let compounder = self.get_strat_mut(&strat_name).get_pemb_mut();
                compounder.treasury.current_amount = 0;
                log!("Transfer to treasure succeeded")
            }
            Err(_) => {
                log!(ERR08_TRANSFER_TO_TREASURE);
            }
        }
    }

    /// Ensure that the transfer to the creator succeeded.
    /// # Parameters example:
    ///   strat_name: pembrock@token_name
    #[private]
    pub fn callback_pembrock_post_creator_ft_transfer(
        &mut self,
        #[callback_result] transfer_result: Result<(), PromiseError>,
        strat_name: String,
    ) {
        match transfer_result {
            Ok(_) => {
                let compounder = self.get_strat_mut(&strat_name).get_pemb_mut();

                // reset strat creator fees after successful transfer
                compounder.strat_creator_fee_amount = 0;

                log!("Transfer to strategy creator succeeded")
            }
            Err(_) => {
                log!(ERR09_TRANSFER_TO_CREATOR);
            }
        }
    }

    /// Transfer an amount of tokens to the sentry contract.
    /// # Parameters example:
    ///   strat_name: pembrock@token_name
    ///   sentry_acc_id: sentry_account.testnet
    ///   reward_token: reward_account.testnet
    #[private]
    pub fn callback_pembrock_post_sentry(
        &mut self,
        #[callback_result] result: Result<Option<StorageBalance>, PromiseError>,
        strat_name: String,
        sentry_acc_id: AccountId,
        reward_token: AccountId,
    ) -> Promise {
        match result {
            Ok(balance_op) => match balance_op {
                Some(balance) => assert!(balance.total.0 > 1),
                _ => {
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

        let compounder = self.get_strat_mut(&strat_name).get_pemb_mut();

        // reset default sentry address and get last earned amount
        let amount = compounder
            .admin_fees
            .sentries
            .remove(&env::current_account_id())
            .unwrap();

        log!("Sending {} to sentry account {}", amount, sentry_acc_id);

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
                callback_pembrock::callback_pembrock_post_sentry_mft_transfer(
                    strat_name.clone(),
                    sentry_acc_id,
                    amount,
                    env::current_account_id(),
                    0,
                    Gas(10_000_000_000_000),
                ),
            );
        }

        ext_ref_exchange::get_return(
            compounder.pool_id_token1_reward,
            compounder.reward_token.clone(),
            U128(compounder.last_reward_amount),
            compounder.token_address.clone(),
            compounder.exchange_contract_id.clone(),
            0,
            Gas(10_000_000_000_000),
        )
        .then(callback_pembrock::callback_pembrock_swap(
            strat_name,
            env::current_account_id(),
            0,
            Gas(180_000_000_000_000),
        ))
    }

    /// Ensure that the transfer succeeded and store the amount earned.
    /// # Parameters example:
    ///   strat_name: pembrock@token_name
    ///   sentry_id: sentry_account.testnet
    ///   amount earned: 10000000
    #[private]
    pub fn callback_pembrock_post_sentry_mft_transfer(
        &mut self,
        #[callback_result] ft_transfer_result: Result<(), PromiseError>,
        strat_name: String,
        sentry_id: AccountId,
        amount_earned: u128,
    ) {
        // in the case where the transfer failed, the next cycle will send it plus the new amount earned
        if ft_transfer_result.is_err() {
            log!(ERR13_TRANSFER_TO_SENTRY);

            let compounder = self.get_strat_mut(&strat_name).get_pemb_mut();

            // store amount earned by sentry to be redeemed
            compounder
                .admin_fees
                .sentries
                .insert(sentry_id, amount_earned);
        } else {
            log!("Transfer to sentry succeeded".to_string());
        }
    }

    /// Swap the tokens and then call a function to lend the new amount of tokens.
    /// # Parameters example:
    ///   strat_name: pembrock@token_name
    #[private]
    pub fn callback_pembrock_swap(
        &mut self,
        #[callback_result] get_return_result: Result<U128, PromiseError>,
        strat_name: String,
    ) -> Promise {
        let strat = self.get_strat(&strat_name);

        let compounder = strat.get_pemb();

        let amount_out = get_return_result.unwrap();

        // apply slippage
        let percent = Percentage::from(compounder.slippage);

        let token_min_out = percent.apply_to(amount_out.0);

        let msg = format!("{{\"force\":0,\"actions\":[{{\"pool_id\":{},\"token_in\":\"{}\",\"token_out\":\"{}\",\"amount_in\":\"{}\",\"min_amount_out\":\"{}\"}}]}}", 461, compounder.reward_token, compounder.token_address, compounder.last_reward_amount, token_min_out) ;

        ext_reward_token::ft_transfer_call(
            compounder.exchange_contract_id,
            U128(compounder.last_reward_amount),
            msg,
            compounder.reward_token,
            1,
            Gas(90_000_000_000_000),
        )
        .then(callback_pembrock::callback_pembrock_lend(
            strat_name,
            env::current_account_id(),
            0,
            Gas(70_000_000_000_000),
        ))
    }

    /// Ensure that the swap succeeded and lend the amount of tokens.
    /// # Parameters example:
    ///   strat_name: pembrock@token_name
    #[private]
    pub fn callback_pembrock_lend(
        &mut self,
        #[callback_result] swap_result: Result<U128, PromiseError>,
        strat_name: String,
    ) -> Promise {
        assert!(swap_result.is_ok(), "{}", ERR10_SWAP_TOKEN);

        let amount_to_transfer = swap_result.unwrap();

        // the total amount of the seed increases
        let total_seed_amount = self.seed_total_amount(&strat_name);

        self.data_mut()
            .seed_id_amount
            .insert(&strat_name, &(total_seed_amount + amount_to_transfer.0));

        let strat = self.get_strat_mut(&strat_name);

        let compounder = strat.get_pemb_mut();

        // after the swap, there's no more reward available to swap
        compounder.last_reward_amount = 0;

        ext_pembrock::ft_transfer_call(
            compounder.pembrock_contract_id.clone(),
            amount_to_transfer,
            "deposit".to_string(),
            compounder.token_address.clone(),
            1,
            Gas(40_000_000_000_000),
        )
        .then(callback_pembrock::callback_pembrock_post_lend(
            strat_name,
            amount_to_transfer.0,
            env::current_account_id(),
            0,
            Gas(10_000_000_000_000),
        ))
    }

    /// Update the harvest available amount to stake.
    /// # Parameters example:
    ///   strat_name: pembrock@token_name
    ///   amount: 10000000
    #[private]
    pub fn callback_pembrock_post_lend(
        &mut self,
        #[callback_result] post_lend_result: Result<U128, PromiseError>,
        strat_name: String,
        amount: u128,
    ) {
        let strat = self.get_strat_mut(&strat_name);

        let compounder = strat.get_pemb_mut();

        if let Ok(_amount) = post_lend_result {
            compounder.harvest_value_available_to_stake = 0;
        } else {
            compounder.harvest_value_available_to_stake += amount;
        }
    }

    /// Withdraw an amount that was being lent and transfer to the caller.
    /// # Parameters example:
    ///   token_name: wrap
    ///   amount_withdraw: U128(10000000) or None
    pub fn pembrock_unstake(
        &mut self,
        token_address: String,
        amount_withdrawal: Option<U128>,
    ) -> Promise {
        let (caller_id, contract_id) = get_predecessor_and_current_account();

        let seed_id: String = format!("pembrock@{}", token_address);

        let fft_share_id = self.get_fft_share_id_from_seed(seed_id.clone());
        let mut user_fft_shares =
            self.users_fft_share_amount(fft_share_id.clone(), caller_id.to_string());

        //Total fft_share
        let total_fft = self.total_supply_amount(fft_share_id);

        //Total seed_id
        let total_seed = self.seed_total_amount(&seed_id);

        //Converting user total fft_shares in seed_id:
        let user_shares = (U256::from(user_fft_shares) * U256::from(total_seed)
            / U256::from(total_fft))
        .as_u128();

        let strat = self
            .data()
            .strategies
            .get(&seed_id)
            .expect(ERR20_SEED_ID_DOES_NOT_EXIST);

        let compounder = strat.clone().get_pemb();

        let amount: U128;
        if let Some(amount_withdrawal) = amount_withdrawal {
            amount = amount_withdrawal;
            user_fft_shares = (U256::from(amount_withdrawal.0) * U256::from(total_fft)
                / U256::from(total_seed))
            .as_u128();
        } else {
            amount = U128(user_shares);
        }
        assert!(
            user_shares >= amount.0,
            "{} is trying to withdrawal {} and only has {}",
            caller_id,
            amount.0,
            user_shares
        );

        log!("{} is trying to withdrawal {}", caller_id, amount.0);

        ext_pembrock::withdraw(
            compounder.token_address.clone(),
            amount,
            compounder.pembrock_contract_id,
            1,
            Gas(100_000_000_000_000),
        )
        .then(ext_reward_token::ft_transfer(
            caller_id.clone(),
            amount,
            Some("".to_string()),
            compounder.token_address,
            1,
            Gas(100_000_000_000_000),
        ))
        .then(callback_ref_finance::callback_withdraw_shares(
            seed_id,
            caller_id,
            amount.0,
            user_fft_shares,
            contract_id,
            0,
            Gas(20_000_000_000_000),
        ))
    }
}
