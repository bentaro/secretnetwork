use cosmwasm_std::{CanonicalAddr, Storage};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

static CONFIG_KEY: &[u8] = b"config";
// static POLL_KEY: &[u8] = b"polls";
static BANK_KEY: &[u8] = b"bank";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub denom: String,
    pub owner: CanonicalAddr,
    pub staked_tokens: u64,
    pub staked_tokens_yes: u64,
    pub staked_tokens_no: u64,
    pub status: PollStatus,
    pub yes_votes: u64,
    pub no_votes: u64,
    pub voters: Vec<CanonicalAddr>,
    pub voter_info: Vec<Voter>,
    pub end_height: u64,
    pub start_height: Option<u64>,
    pub description: String,

}

#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenManager {
    pub token_balance: u64,             // total staked balance
    pub vote: String,                       // vote info ("yes" or "no")
    // pub locked_tokens: Vec<(u64, Uint128)>, //maps poll_id to weight voted
    // pub participated_polls: Vec<u64>,       // poll_id
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Voter {
    pub vote: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum PollStatus {
    InProgress,
    Yes,
    No,
    Rejected,
}

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// pub struct Poll {
//     pub creator: CanonicalAddr,
//     pub status: PollStatus,
//     pub quorum_percentage: Option<u8>,
//     pub yes_votes: Uint128,
//     pub no_votes: Uint128,
//     pub voters: Vec<CanonicalAddr>,
//     pub voter_info: Vec<Voter>,
//     pub end_height: u64,
//     pub start_height: Option<u64>,
//     pub description: String,
// }

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, CONFIG_KEY)
}

// pub fn poll<S: Storage>(storage: &mut S) -> Bucket<S, Poll> {
//     bucket(POLL_KEY, storage)
// }
//
// pub fn poll_read<S: Storage>(storage: &S) -> ReadonlyBucket<S, Poll> {
//     bucket_read(POLL_KEY, storage)
// }

pub fn bank<S: Storage>(storage: &mut S) -> Bucket<S, TokenManager> {
    bucket(BANK_KEY, storage)
}

pub fn bank_read<S: Storage>(storage: &S) -> ReadonlyBucket<S, TokenManager> {
    bucket_read(BANK_KEY, storage)
}
