// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::types::ContractCodeId;
use crate::ValidatorClientError;
use async_trait::async_trait;
use cosmos_sdk::proto::cosmos::auth::v1beta1::{
    BaseAccount, QueryAccountRequest, QueryAccountResponse,
};
use cosmos_sdk::proto::cosmos::bank::v1beta1::{
    QueryAllBalancesRequest, QueryAllBalancesResponse, QueryBalanceRequest, QueryBalanceResponse,
};
use cosmos_sdk::proto::cosmwasm::wasm::v1beta1::*;
use cosmos_sdk::rpc::endpoint::block::Response as BlockResponse;
use cosmos_sdk::rpc::endpoint::broadcast;
use cosmos_sdk::rpc::endpoint::tx::Response as TxResponse;
use cosmos_sdk::rpc::endpoint::tx_search::Response as TxSearchResponse;
use cosmos_sdk::rpc::query::Query;
use cosmos_sdk::rpc::{
    Client, Error as TendermintRpcError, HttpClient, HttpClientUrl, SimpleRequest,
};
use cosmos_sdk::tendermint::abci::Transaction;
use cosmos_sdk::tendermint::{abci, block, chain};
use cosmos_sdk::tx::{AccountNumber, SequenceNumber};
use cosmos_sdk::{rpc, AccountId, Coin, Denom};
use prost::Message;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;

// #[async_trait]
pub struct CosmWasmClient {
    tm_client: HttpClient,
}

type TodoType = ();

impl CosmWasmClient {
    pub fn connect<U>(endpoint: U) -> Result<Self, ValidatorClientError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let tm_client = HttpClient::new(endpoint)?;

        Ok(CosmWasmClient { tm_client })
    }

    async fn make_abci_query<Req, Res>(
        &self,
        path: Option<abci::Path>,
        req: Req,
    ) -> Result<Res, ValidatorClientError>
    where
        Req: Message,
        Res: Message + Default,
    {
        let mut buf = Vec::with_capacity(req.encoded_len());
        req.encode(&mut buf)?;

        let res = self.tm_client.abci_query(path, buf, None, false).await?;

        Ok(Res::decode(res.value.as_ref())?)
    }

    pub async fn get_chain_id(&self) -> Result<chain::Id, ValidatorClientError> {
        Ok(self.tm_client.status().await?.node_info.network)
    }

    pub async fn get_height(&self) -> Result<block::Height, ValidatorClientError> {
        Ok(self.tm_client.status().await?.sync_info.latest_block_height)
    }

    // TODO: the return type should probably be changed to a non-proto, type-safe Account alternative
    pub async fn get_account(
        &self,
        address: &AccountId,
    ) -> Result<Option<BaseAccount>, ValidatorClientError> {
        let path = Some("/cosmos.auth.v1beta1.Query/Account".parse().unwrap());

        let req = QueryAccountRequest {
            address: address.to_string(),
        };

        let res = self
            .make_abci_query::<_, QueryAccountResponse>(path, req)
            .await?;

        let base_account = res
            .account
            .map(|account| BaseAccount::decode(account.value.as_ref()))
            .transpose()?;

        Ok(base_account)
    }

    pub async fn get_sequence(
        &self,
        address: &AccountId,
    ) -> Result<(AccountNumber, SequenceNumber), ValidatorClientError> {
        let base_account = self
            .get_account(address)
            .await?
            .ok_or_else(|| ValidatorClientError::NonExistentAccountError(address.clone()))?;
        Ok((base_account.account_number, base_account.sequence))
    }

    pub async fn get_block(
        &self,
        height: Option<u32>,
    ) -> Result<BlockResponse, ValidatorClientError> {
        match height {
            Some(height) => self.tm_client.block(height).await.map_err(|err| err.into()),
            None => self
                .tm_client
                .latest_block()
                .await
                .map_err(|err| err.into()),
        }
    }

    pub async fn get_balance(
        &self,
        address: &AccountId,
        search_denom: Denom,
    ) -> Result<Option<Coin>, ValidatorClientError> {
        let path = Some("/cosmos.bank.v1beta1.Query/Balance".parse().unwrap());

        let req = QueryBalanceRequest {
            address: address.to_string(),
            denom: search_denom.to_string(),
        };

        let res = self
            .make_abci_query::<_, QueryBalanceResponse>(path, req)
            .await?;

        res.balance
            .map(TryFrom::try_from)
            .transpose()
            .map_err(|_| ValidatorClientError::SerializationError("Coin".to_owned()))
    }

    // TODO: if this is to be made into more generic library, it will need to be able to handle
    // pagination
    pub async fn get_all_balances(
        &self,
        address: &AccountId,
    ) -> Result<Vec<Coin>, ValidatorClientError> {
        let path = Some("/cosmos.bank.v1beta1.Query/AllBalances".parse().unwrap());

        let req = QueryAllBalancesRequest {
            address: address.to_string(),
            pagination: None,
        };

        let res = self
            .make_abci_query::<_, QueryAllBalancesResponse>(path, req)
            .await?;

        res.balances
            .into_iter()
            .map(TryFrom::try_from)
            .collect::<Result<_, _>>()
            .map_err(|_| ValidatorClientError::SerializationError("Coins".to_owned()))
    }

    // TODO: or should it instead take concrete Hash type directly?
    pub async fn get_tx(&self, id: &str) -> Result<TxResponse, ValidatorClientError> {
        let tx_hash = id
            .parse()
            .map_err(|_| ValidatorClientError::InvalidTxHash(id.to_owned()))?;
        Ok(self.tm_client.tx(tx_hash, false).await?)
    }

    pub async fn search_tx(&self, query: Query) -> Result<TxSearchResponse, ValidatorClientError> {
        todo!("need to construct pagination here")
        // self.http_client.tx_search(query, false, )
        /*
        /// `/tx_search`: search for transactions with their results.
        async fn tx_search(
            &self,
            query: Query,
            prove: bool,
            page: u32,
            per_page: u8,
            order: Order,
        ) -> Result<tx_search::Response> {
            self.perform(tx_search::Request::new(query, prove, page, per_page, order))
                .await
        }
         */
    }

    /// Broadcast a transaction, returning immediately.
    pub async fn broadcast_tx_async(
        &self,
        tx: Transaction,
    ) -> Result<broadcast::tx_async::Response, ValidatorClientError> {
        Ok(self.tm_client.broadcast_tx_async(tx).await?)
    }

    /// Broadcast a transaction, returning the response from `CheckTx`.
    pub async fn broadcast_tx_sync(
        &self,
        tx: Transaction,
    ) -> Result<broadcast::tx_sync::Response, ValidatorClientError> {
        Ok(self.tm_client.broadcast_tx_sync(tx).await?)
    }

    /// Broadcast a transaction, returning the response from `DeliverTx`.
    pub async fn broadcast_tx_commit(
        &self,
        tx: Transaction,
    ) -> Result<broadcast::tx_commit::Response, ValidatorClientError> {
        Ok(self.tm_client.broadcast_tx_commit(tx).await?)
    }

    pub async fn get_codes(&self) -> Result<TodoType, ValidatorClientError> {
        let path = Some("/cosmwasm.wasm.v1beta1.Query/Codes".parse().unwrap());

        let req = QueryCodesRequest { pagination: None };

        let res = self
            .make_abci_query::<_, QueryCodesResponse>(path, req)
            .await?;

        todo!("some CodeInfoResponse parsing action here")
    }

    pub async fn get_code_details(
        &self,
        code_id: ContractCodeId,
    ) -> Result<TodoType, ValidatorClientError> {
        let path = Some("/cosmwasm.wasm.v1beta1.Query/Code".parse().unwrap());

        let req = QueryCodeRequest { code_id };

        let res = self
            .make_abci_query::<_, QueryCodeResponse>(path, req)
            .await?;

        todo!("some CodeInfoResponse parsing action here")
    }
    pub async fn get_contracts(
        &self,
        code_id: ContractCodeId,
    ) -> Result<Vec<AccountId>, ValidatorClientError> {
        let path = Some(
            "/cosmwasm.wasm.v1beta1.Query/ContractsByCode"
                .parse()
                .unwrap(),
        );

        let req = QueryContractsByCodeRequest {
            code_id,
            pagination: None,
        };

        // TODO: pagination...
        let res = self
            .make_abci_query::<_, QueryContractsByCodeResponse>(path, req)
            .await?;

        res.contracts
            .iter()
            .map(|raw| raw.parse())
            .collect::<Result<_, _>>()
            .map_err(|_| {
                ValidatorClientError::DeserializationError("Contract addresses".to_owned())
            })
    }

    pub async fn get_contract(
        &self,
        address: &AccountId,
    ) -> Result<TodoType, ValidatorClientError> {
        let path = Some("/cosmwasm.wasm.v1beta1.Query/ContractInfo".parse().unwrap());

        // TODO: pagination....
        let req = QueryContractInfoRequest {
            address: address.to_string(),
        };

        let res = self
            .make_abci_query::<_, QueryContractInfoResponse>(path, req)
            .await?;

        todo!("some ContractInfo parsing action here")
    }

    pub async fn get_contract_code_history(
        &self,
        address: &AccountId,
    ) -> Result<TodoType, ValidatorClientError> {
        let path = Some(
            "/cosmwasm.wasm.v1beta1.Query/ContractHistory"
                .parse()
                .unwrap(),
        );

        // TODO: pagination....
        let req = QueryContractHistoryRequest {
            address: address.to_string(),
            pagination: None,
        };

        let res = self
            .make_abci_query::<_, QueryContractHistoryResponse>(path, req)
            .await?;

        todo!("some ContractCodeHistoryEntry parsing action here")
    }

    pub async fn query_contract_raw(
        &self,
        address: &AccountId,
        query_data: Vec<u8>,
    ) -> Result<Vec<u8>, ValidatorClientError> {
        let path = Some(
            "/cosmwasm.wasm.v1beta1.Query/RawContractState"
                .parse()
                .unwrap(),
        );

        let req = QueryRawContractStateRequest {
            address: address.to_string(),
            query_data,
        };

        let res = self
            .make_abci_query::<_, QueryRawContractStateResponse>(path, req)
            .await?;

        Ok(res.data)
    }

    pub async fn query_contract_smart<M, T>(
        &self,
        address: &AccountId,
        query_msg: &M,
    ) -> Result<T, ValidatorClientError>
    where
        M: ?Sized + Serialize,
        for<'a> T: Deserialize<'a>,
    {
        let path = Some(
            "/cosmwasm.wasm.v1beta1.Query/SmartContractState"
                .parse()
                .unwrap(),
        );

        // As per serde documentation:
        // Serialization can fail if `T`'s implementation of `Serialize` decides to
        // fail, or if `T` contains a map with non-string keys.
        let req = QuerySmartContractStateRequest {
            address: address.to_string(),
            query_data: serde_json::to_vec(query_msg)?,
        };

        let res = self
            .make_abci_query::<_, QuerySmartContractStateResponse>(path, req)
            .await?;

        Ok(serde_json::from_slice(&res.data)?)
    }
}

// #[async_trait]
// impl QueryCosmWasmClient for Client {}
//
// #[async_trait]
// impl rpc::Client for Client {
//     async fn perform<R>(&self, request: R) -> rpc::Result<R::Response>
//     where
//         R: SimpleRequest,
//     {
//         self.http_client.perform(request).await
//     }
// }
