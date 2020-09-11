use crate::coin_helpers::assert_sent_sufficient_coin;
use crate::msg::{
    HandleMsg, InitMsg, QueryMsg, TokenStakeResponse,
};
use crate::state::{
    bank, bank_read, config, config_read, PollStatus, State, Voter,
};
use cosmwasm_std::{
    coin, log, to_binary, Api, BankMsg, Binary, CanonicalAddr, Coin, CosmosMsg, Env, Extern,
    HandleResponse, HandleResult, HumanAddr, InitResponse, InitResult, Querier, StdError,
    StdResult, Storage,
};

pub const VOTING_TOKEN: &str = "voting_token";
pub const DEFAULT_END_HEIGHT_BLOCKS: &u64 = &50_000_u64;
const MIN_STAKE_AMOUNT: u128 = 1;
// const MIN_DESC_LENGTH: usize = 3;
// const MAX_DESC_LENGTH: usize = 64;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> InitResult {
    let state = State {
        denom: msg.denom,
        owner: deps.api.canonical_address(&env.message.sender)?,
        staked_tokens: 0,
        staked_tokens_yes: 0,
        staked_tokens_no: 0,

        //added
        status: PollStatus::InProgress,
        yes_votes: 0,
        no_votes: 0,
        voters: vec![],
        voter_info: vec![],
        start_height: msg.start_height,
        end_height: msg.end_height,
        description: msg.description,

    };
    config(&mut deps.storage).save(&state)?;

    Ok(InitResponse::default())
}


pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::StakeAndVote {
            vote,
        } => stake_and_vote(deps, env, vote),

        HandleMsg::WithdrawVotingTokens {} => withdraw_voting_tokens(deps, env),

        HandleMsg::EndPoll {} => end_poll(deps, env),
    }
}


// integrate stake_voting_tokens and cast_vote

pub fn stake_and_vote<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    vote: String,
) -> HandleResult {
    let sender_address_raw = deps.api.canonical_address(&env.message.sender)?;
    let key = &sender_address_raw.as_slice();

    let mut token_manager = bank_read(&deps.storage).may_load(key)?.unwrap_or_default();

    let mut state = config(&mut deps.storage).load()?;

    //whether voting is active or inactive
    if state.status != PollStatus::InProgress {
        return Err(StdError::generic_err("Poll is not in progress"));
    }
    //whether user has voted or not
    if has_voted(&sender_address_raw, &state) {
        return Err(StdError::generic_err("User has already voted."));
    }

    assert_sent_sufficient_coin(
        &env.message.sent_funds,
        Some(coin(MIN_STAKE_AMOUNT, &state.denom)),
    )?;
    let sent_funds = env
        .message
        .sent_funds
        .iter()
        .find(|coin| coin.denom.eq(&state.denom))
        .unwrap();

    let sent_amount = sent_funds.amount.u128() as u64;
    token_manager.token_balance = sent_amount;
    token_manager.vote = vote.clone();

    let staked_tokens = state.staked_tokens + sent_amount;
    state.staked_tokens = staked_tokens;

    if vote=="yes" {
        state.staked_tokens_yes += sent_amount;
        state.yes_votes += 1;
    }else{
        state.staked_tokens_no += sent_amount;
        state.no_votes += 1
    }

    //add voters and voter_info
    state.voters.push(sender_address_raw.clone());
    let voter_info = Voter { vote };
    state.voter_info.push(voter_info);

    config(&mut deps.storage).save(&state)?;

    bank(&mut deps.storage).save(key, &token_manager)?;

    let log = vec![
        log("action", "vote_casted"),
        log("voter", &env.message.sender.as_str()),
    ];

    let r = HandleResponse {
        messages: vec![],
        log,
        data: None,
    };
    Ok(r)
}

fn has_voted(voter: &CanonicalAddr, state: &State) -> bool {
    state.voters.iter().any(|i| i == voter)
}

// Withdraw amount if not staked. By default all funds will be withdrawn.
pub fn withdraw_voting_tokens<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let sender_address_raw = deps.api.canonical_address(&env.message.sender)?;
    let contract_address_raw = deps.api.canonical_address(&env.contract.address)?;
    let key = sender_address_raw.as_slice();

    let state = config(&mut deps.storage).load()?;

    // if let Pollstatus::InProgress=state.status {
    //     Err(StdError::generic_err(
    //         "User is trying to withdraw tokens while voting is active.",
    //     ))
    // }


    if let Some(mut token_manager) = bank_read(&deps.storage).may_load(key)? {
        // // let largest_staked = locked_amount(&sender_address_raw, deps);
        // let withdraw_amount = match amount {
        //     Some(amount) => Some(amount.u128()),
        //     None => Some(token_manager.token_balance.u128()),
        // }
        // .unwrap();

        let staked_tokens_total = state.staked_tokens as u128;
        let staked_tokens_winner;

        match state.status {
            PollStatus::Yes => {
                if token_manager.vote!="yes".to_string(){
                    return Err(StdError::generic_err(
                        "User is trying to withdraw tokens whereas he lost.",
                    ));
                }
                staked_tokens_winner = state.staked_tokens_yes as u128;
            },
            PollStatus::No =>{
                if token_manager.vote=="yes".to_string(){
                    return Err(StdError::generic_err(
                        "User is trying to withdraw tokens whereas he lost.",
                    ));
                }
                staked_tokens_winner = state.staked_tokens_no as u128;
            },
            _ =>{
                return Err(StdError::generic_err(
                        "User is trying to withdraw tokens while voting is active.",
                    ));
                }
        }
        //cast each member for calculation withdrawn balance
        let balance = token_manager.token_balance as u128;
        let balance = balance * staked_tokens_total / staked_tokens_winner;

        token_manager.token_balance = 0;

        bank(&mut deps.storage).save(key, &token_manager)?;
        config(&mut deps.storage).save(&state)?;

        send_tokens(
            &deps.api,
            &contract_address_raw,
            &sender_address_raw,
            vec![coin(balance, &state.denom)],
            "approve",
            )
    } else {
        Err(StdError::generic_err("Nothing staked"))
    }
}

/// validate_description returns an error if the description is invalid
// fn validate_description(description: &str) -> StdResult<()> {
//     if description.len() < MIN_DESC_LENGTH {
//         Err(StdError::generic_err("Description too short"))
//     } else if description.len() > MAX_DESC_LENGTH {
//         Err(StdError::generic_err("Description too long"))
//     } else {
//         Ok(())
//     }
// }

/// validate_quorum_percentage returns an error if the quorum_percentage is invalid
/// (we require 0-100)
// fn validate_quorum_percentage(quorum_percentage: Option<u8>) -> StdResult<()> {
//     if quorum_percentage.is_some() && quorum_percentage.unwrap() > 100 {
//         Err(StdError::generic_err("quorum_percentage must be 0 to 100"))
//     } else {
//         Ok(())
//     }
// }

/// validate_end_height returns an error if the poll ends in the past
// fn validate_end_height(end_height: Option<u64>, env: Env) -> StdResult<()> {
//     if end_height.is_some() && env.block.height >= end_height.unwrap() {
//         Err(StdError::generic_err("Poll cannot end in the past"))
//     } else {
//         Ok(())
//     }
// }


/*
 * Ends a poll. Only the creator of a given poll can end that poll.
 */
pub fn end_poll<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let mut state = config(&mut deps.storage).load()?;

    let sender_address_raw = deps.api.canonical_address(&env.message.sender)?;
    if state.owner != sender_address_raw {
        return Err(StdError::generic_err(
            "User is not the creator of the poll.",
        ));
    }

    if state.status != PollStatus::InProgress {
        return Err(StdError::generic_err("Poll is not in progress"));
    }

    if state.start_height.is_some() && state.start_height.unwrap() > env.block.height {
        return Err(StdError::generic_err("Voting period has not started."));
    }

    if state.end_height > env.block.height {
        return Err(StdError::generic_err("Voting period has not expired."));
    }
    let yes = state.yes_votes;
    let  no = state.no_votes;

    //suppose either votes are 0 or same amount of votes, voting rejected
    if yes*no==0 || yes==no{
        state.status = PollStatus::Rejected;
    }
    //suppose yes votes are greater than no, change status to No(majority lose)
    else if yes < no{
        state.status = PollStatus::Yes;
    }else{
        state.status = PollStatus::No;
    }

    config(&mut deps.storage).save(&state)?;

    let log = vec![
        log("action", "end_poll"),
    ];

    let r = HandleResponse {
        messages: vec![],
        log,
        data: None,
    };
    Ok(r)
}

fn send_tokens<A: Api>(
    api: &A,
    from_address: &CanonicalAddr,
    to_address: &CanonicalAddr,
    amount: Vec<Coin>,
    action: &str,
) -> HandleResult {
    let from_human = api.human_address(from_address)?;
    let to_human = api.human_address(to_address)?;
    let log = vec![log("action", action), log("to", to_human.as_str())];

    let r = HandleResponse {
        messages: vec![CosmosMsg::Bank(BankMsg::Send {
            from_address: from_human,
            to_address: to_human,
            amount,
        })],
        log,
        data: None,
    };
    Ok(r)
}

//クエリ値をバイナリデータとして返す
pub fn query<S: Storage, A: Api, Q: Querier>(
    _deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&config_read(&_deps.storage).load()?),

        QueryMsg::TokenStake { address } => token_balance(_deps, address),
    }
}

fn token_balance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: HumanAddr,
) -> StdResult<Binary> {
    let key = deps.api.canonical_address(&address).unwrap();

    let token_manager = bank_read(&deps.storage)
        .may_load(key.as_slice())?
        .unwrap_or_default();

    let resp = TokenStakeResponse {
        token_balance: token_manager.token_balance,
    };

    to_binary(&resp)
}
