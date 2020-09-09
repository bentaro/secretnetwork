#[cfg(test)]
mod tests {
    use crate::contract::{handle, init, query, VOTING_TOKEN};
    use crate::msg::{HandleMsg, InitMsg, QueryMsg};
    use crate::state::{config_read, State, PollStatus};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{
        coins, from_binary, log, Api, BankMsg, Coin, CosmosMsg, Env, Extern, HandleResponse,
        HumanAddr, StdError, Uint128,
    };

    const DEFAULT_END_HEIGHT: u64 = 100800u64;
    const TEST_CREATOR: &str = "creator";
    const TEST_VOTER: &str = "voter1";
    const TEST_VOTER_2: &str = "voter2";

    fn init_msg() -> InitMsg {
        InitMsg {
            denom: String::from(VOTING_TOKEN),
            start_height: Some(100_000_u64),
            end_height: 80_000_u64,
            description: String::from("test")
        }
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = init_msg();
        let env = mock_env(TEST_CREATOR, &coins(2, VOTING_TOKEN));
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let state = config_read(&mut deps.storage).load().unwrap();
        assert_eq!(
            state,
            State {
                denom: String::from(VOTING_TOKEN),
                owner: deps
                    .api
                    .canonical_address(&HumanAddr::from(TEST_CREATOR))
                    .unwrap(),
                staked_tokens: 0,
                staked_tokens_yes: 0,
                staked_tokens_no: 0,
                status: PollStatus::InProgress,
                yes_votes: 0,
                no_votes: 0,
                voters: vec![],
                voter_info: vec![],
                end_height: 80000,
                start_height: Some(100000),
                description: "test".to_string(),
            }
        );
    }
}
