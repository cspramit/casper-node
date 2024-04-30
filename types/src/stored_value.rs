mod global_state_identifier;
mod type_mismatch;

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use core::{convert::TryFrom, fmt::Debug};

#[cfg(feature = "datasize")]
use datasize::DataSize;
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;
use serde::{de, ser, Deserialize, Deserializer, Serialize, Serializer};
use serde_bytes::ByteBuf;

use crate::{
    account::Account,
    addressable_entity::NamedKeyValue,
    bytesrepr::{self, Error, FromBytes, ToBytes, U8_SERIALIZED_LENGTH},
    contract_messages::{MessageChecksum, MessageTopicSummary},
    contract_wasm::ContractWasm,
    contracts::{Contract, ContractPackage},
    package::Package,
    system::{
        auction::{Bid, BidKind, EraInfo, UnbondingPurse, WithdrawPurse},
        reservations::ReservationKind,
    },
    AddressableEntity, ByteCode, CLValue, DeployInfo, TransferV1,
};
pub use global_state_identifier::GlobalStateIdentifier;
pub use type_mismatch::TypeMismatch;

#[allow(clippy::large_enum_variant)]
#[repr(u8)]
enum Tag {
    CLValue = 0,
    Account = 1,
    ContractWasm = 2,
    Contract = 3,
    ContractPackage = 4,
    LegacyTransfer = 5,
    DeployInfo = 6,
    EraInfo = 7,
    Bid = 8,
    Withdraw = 9,
    Unbonding = 10,
    BidKind = 11,
    Package = 12,
    AddressableEntity = 13,
    ByteCode = 14,
    MessageTopic = 15,
    Message = 16,
    NamedKey = 17,
    Reservation = 18,
}

/// A value stored in Global State.
#[allow(clippy::large_enum_variant)]
#[derive(Eq, PartialEq, Clone, Debug)]
#[cfg_attr(feature = "datasize", derive(DataSize))]
#[cfg_attr(
    feature = "json-schema",
    derive(JsonSchema),
    schemars(with = "serde_helpers::HumanReadableSerHelper")
)]
pub enum StoredValue {
    /// A CLValue.
    CLValue(CLValue),
    /// An account.
    Account(Account),
    /// Contract wasm.
    ContractWasm(ContractWasm),
    /// A contract.
    Contract(Contract),
    /// A contract package.
    ContractPackage(ContractPackage),
    /// A version 1 (legacy) transfer.
    LegacyTransfer(TransferV1),
    /// Info about a deploy.
    DeployInfo(DeployInfo),
    /// Info about an era.
    EraInfo(EraInfo),
    /// Variant that stores [`Bid`].
    Bid(Box<Bid>),
    /// Variant that stores withdraw information.
    Withdraw(Vec<WithdrawPurse>),
    /// Unbonding information.
    Unbonding(Vec<UnbondingPurse>),
    /// An `AddressableEntity`.
    AddressableEntity(AddressableEntity),
    /// Variant that stores [`BidKind`].
    BidKind(BidKind),
    /// A `Package`.
    Package(Package),
    /// A record of byte code.
    ByteCode(ByteCode),
    /// Variant that stores a message topic.
    MessageTopic(MessageTopicSummary),
    /// Variant that stores a message digest.
    Message(MessageChecksum),
    /// A NamedKey record.
    NamedKey(NamedKeyValue),
    /// A reservation record.
    Reservation(ReservationKind),
}

impl StoredValue {
    /// Returns a reference to the wrapped `CLValue` if this is a `CLValue` variant.
    pub fn as_cl_value(&self) -> Option<&CLValue> {
        match self {
            StoredValue::CLValue(cl_value) => Some(cl_value),
            _ => None,
        }
    }

    /// Returns a reference to the wrapped `Account` if this is an `Account` variant.
    pub fn as_account(&self) -> Option<&Account> {
        match self {
            StoredValue::Account(account) => Some(account),
            _ => None,
        }
    }

    /// Returns a reference to the wrapped `ByteCode` if this is a `ByteCode` variant.
    pub fn as_byte_code(&self) -> Option<&ByteCode> {
        match self {
            StoredValue::ByteCode(byte_code) => Some(byte_code),
            _ => None,
        }
    }

    /// Returns a reference to the wrapped `Contract` if this is a `Contract` variant.
    pub fn as_contract(&self) -> Option<&Contract> {
        match self {
            StoredValue::Contract(contract) => Some(contract),
            _ => None,
        }
    }

    /// Returns a reference to the wrapped `Package` if this is a `Package` variant.
    pub fn as_package(&self) -> Option<&Package> {
        match self {
            StoredValue::Package(package) => Some(package),
            _ => None,
        }
    }

    /// Returns a reference to the wrapped `TransferV1` if this is a `LegacyTransfer` variant.
    pub fn as_legacy_transfer(&self) -> Option<&TransferV1> {
        match self {
            StoredValue::LegacyTransfer(transfer_v1) => Some(transfer_v1),
            _ => None,
        }
    }

    /// Returns a reference to the wrapped `DeployInfo` if this is a `DeployInfo` variant.
    pub fn as_deploy_info(&self) -> Option<&DeployInfo> {
        match self {
            StoredValue::DeployInfo(deploy_info) => Some(deploy_info),
            _ => None,
        }
    }

    /// Returns a reference to the wrapped `EraInfo` if this is an `EraInfo` variant.
    pub fn as_era_info(&self) -> Option<&EraInfo> {
        match self {
            StoredValue::EraInfo(era_info) => Some(era_info),
            _ => None,
        }
    }

    /// Returns a reference to the wrapped `Bid` if this is a `Bid` variant.
    pub fn as_bid(&self) -> Option<&Bid> {
        match self {
            StoredValue::Bid(bid) => Some(bid),
            _ => None,
        }
    }

    /// Returns a reference to the wrapped list of `WithdrawPurse`s if this is a `Withdraw` variant.
    pub fn as_withdraw(&self) -> Option<&Vec<WithdrawPurse>> {
        match self {
            StoredValue::Withdraw(withdraw_purses) => Some(withdraw_purses),
            _ => None,
        }
    }

    /// Returns a reference to the wrapped list of `UnbondingPurse`s if this is an `Unbonding`
    /// variant.
    pub fn as_unbonding(&self) -> Option<&Vec<UnbondingPurse>> {
        match self {
            StoredValue::Unbonding(unbonding_purses) => Some(unbonding_purses),
            _ => None,
        }
    }

    /// Returns a reference to the wrapped `AddressableEntity` if this is an `AddressableEntity`
    /// variant.
    pub fn as_addressable_entity(&self) -> Option<&AddressableEntity> {
        match self {
            StoredValue::AddressableEntity(entity) => Some(entity),
            _ => None,
        }
    }

    /// Returns a reference to the wrapped `MessageTopicSummary` if this is a `MessageTopic`
    /// variant.
    pub fn as_message_topic_summary(&self) -> Option<&MessageTopicSummary> {
        match self {
            StoredValue::MessageTopic(summary) => Some(summary),
            _ => None,
        }
    }

    /// Returns a reference to the wrapped `MessageChecksum` if this is a `Message`
    /// variant.
    pub fn as_message_checksum(&self) -> Option<&MessageChecksum> {
        match self {
            StoredValue::Message(checksum) => Some(checksum),
            _ => None,
        }
    }

    /// Returns a reference to the wrapped `BidKind` if this is a `BidKind` variant.
    pub fn as_bid_kind(&self) -> Option<&BidKind> {
        match self {
            StoredValue::BidKind(bid_kind) => Some(bid_kind),
            _ => None,
        }
    }

    /// Returns the `CLValue` if this is a `CLValue` variant.
    pub fn into_cl_value(self) -> Option<CLValue> {
        match self {
            StoredValue::CLValue(cl_value) => Some(cl_value),
            _ => None,
        }
    }

    /// Returns the `Account` if this is an `Account` variant.
    pub fn into_account(self) -> Option<Account> {
        match self {
            StoredValue::Account(account) => Some(account),
            _ => None,
        }
    }

    /// Returns the `ContractWasm` if this is a `ContractWasm` variant.
    pub fn into_contract_wasm(self) -> Option<ContractWasm> {
        match self {
            StoredValue::ContractWasm(contract_wasm) => Some(contract_wasm),
            _ => None,
        }
    }

    /// Returns the `Contract` if this is a `Contract` variant.
    pub fn into_contract(self) -> Option<Contract> {
        match self {
            StoredValue::Contract(contract) => Some(contract),
            _ => None,
        }
    }

    /// Returns the `ContractPackage` if this is a `ContractPackage` variant.
    pub fn into_contract_package(self) -> Option<ContractPackage> {
        match self {
            StoredValue::ContractPackage(contract_package) => Some(contract_package),
            _ => None,
        }
    }

    /// Returns the `Package` if this is a `Package` variant.
    pub fn into_package(self) -> Option<Package> {
        match self {
            StoredValue::Package(package) => Some(package),
            _ => None,
        }
    }

    /// Returns the `TransferV1` if this is a `LegacyTransfer` variant.
    pub fn into_legacy_transfer(self) -> Option<TransferV1> {
        match self {
            StoredValue::LegacyTransfer(transfer_v1) => Some(transfer_v1),
            _ => None,
        }
    }

    /// Returns the `DeployInfo` if this is a `DeployInfo` variant.
    pub fn into_deploy_info(self) -> Option<DeployInfo> {
        match self {
            StoredValue::DeployInfo(deploy_info) => Some(deploy_info),
            _ => None,
        }
    }

    /// Returns the `EraInfo` if this is an `EraInfo` variant.
    pub fn into_era_info(self) -> Option<EraInfo> {
        match self {
            StoredValue::EraInfo(era_info) => Some(era_info),
            _ => None,
        }
    }

    /// Returns the `Bid` if this is a `Bid` variant.
    pub fn into_bid(self) -> Option<Bid> {
        match self {
            StoredValue::Bid(bid) => Some(*bid),
            _ => None,
        }
    }

    /// Returns the list of `WithdrawPurse`s if this is a `Withdraw` variant.
    pub fn into_withdraw(self) -> Option<Vec<WithdrawPurse>> {
        match self {
            StoredValue::Withdraw(withdraw_purses) => Some(withdraw_purses),
            _ => None,
        }
    }

    /// Returns the list of `UnbondingPurse`s if this is an `Unbonding` variant.
    pub fn into_unbonding(self) -> Option<Vec<UnbondingPurse>> {
        match self {
            StoredValue::Unbonding(unbonding_purses) => Some(unbonding_purses),
            _ => None,
        }
    }

    /// Returns the `AddressableEntity` if this is an `AddressableEntity` variant.
    pub fn into_addressable_entity(self) -> Option<AddressableEntity> {
        match self {
            StoredValue::AddressableEntity(entity) => Some(entity),
            _ => None,
        }
    }

    /// Returns the `BidKind` if this is a `BidKind` variant.
    pub fn into_bid_kind(self) -> Option<BidKind> {
        match self {
            StoredValue::BidKind(bid_kind) => Some(bid_kind),
            _ => None,
        }
    }

    /// Returns the type name of the [`StoredValue`] enum variant.
    ///
    /// For [`CLValue`] variants it will return the name of the [`CLType`](crate::cl_type::CLType)
    pub fn type_name(&self) -> String {
        match self {
            StoredValue::CLValue(cl_value) => format!("{:?}", cl_value.cl_type()),
            StoredValue::Account(_) => "Account".to_string(),
            StoredValue::ContractWasm(_) => "ContractWasm".to_string(),
            StoredValue::Contract(_) => "Contract".to_string(),
            StoredValue::ContractPackage(_) => "ContractPackage".to_string(),
            StoredValue::LegacyTransfer(_) => "LegacyTransfer".to_string(),
            StoredValue::DeployInfo(_) => "DeployInfo".to_string(),
            StoredValue::EraInfo(_) => "EraInfo".to_string(),
            StoredValue::Bid(_) => "Bid".to_string(),
            StoredValue::Withdraw(_) => "Withdraw".to_string(),
            StoredValue::Unbonding(_) => "Unbonding".to_string(),
            StoredValue::AddressableEntity(_) => "AddressableEntity".to_string(),
            StoredValue::BidKind(_) => "BidKind".to_string(),
            StoredValue::ByteCode(_) => "ByteCode".to_string(),
            StoredValue::Package(_) => "Package".to_string(),
            StoredValue::MessageTopic(_) => "MessageTopic".to_string(),
            StoredValue::Message(_) => "Message".to_string(),
            StoredValue::NamedKey(_) => "NamedKey".to_string(),
            StoredValue::Reservation(_) => "Reservation".to_string(),
        }
    }

    fn tag(&self) -> Tag {
        match self {
            StoredValue::CLValue(_) => Tag::CLValue,
            StoredValue::Account(_) => Tag::Account,
            StoredValue::ContractWasm(_) => Tag::ContractWasm,
            StoredValue::ContractPackage(_) => Tag::ContractPackage,
            StoredValue::Contract(_) => Tag::Contract,
            StoredValue::LegacyTransfer(_) => Tag::LegacyTransfer,
            StoredValue::DeployInfo(_) => Tag::DeployInfo,
            StoredValue::EraInfo(_) => Tag::EraInfo,
            StoredValue::Bid(_) => Tag::Bid,
            StoredValue::Withdraw(_) => Tag::Withdraw,
            StoredValue::Unbonding(_) => Tag::Unbonding,
            StoredValue::AddressableEntity(_) => Tag::AddressableEntity,
            StoredValue::BidKind(_) => Tag::BidKind,
            StoredValue::Package(_) => Tag::Package,
            StoredValue::ByteCode(_) => Tag::ByteCode,
            StoredValue::MessageTopic(_) => Tag::MessageTopic,
            StoredValue::Message(_) => Tag::Message,
            StoredValue::NamedKey(_) => Tag::NamedKey,
            StoredValue::Reservation(_) => Tag::Reservation,
        }
    }
}

impl From<CLValue> for StoredValue {
    fn from(value: CLValue) -> StoredValue {
        StoredValue::CLValue(value)
    }
}
impl From<Account> for StoredValue {
    fn from(value: Account) -> StoredValue {
        StoredValue::Account(value)
    }
}

impl From<ContractWasm> for StoredValue {
    fn from(value: ContractWasm) -> Self {
        StoredValue::ContractWasm(value)
    }
}

impl From<ContractPackage> for StoredValue {
    fn from(value: ContractPackage) -> Self {
        StoredValue::ContractPackage(value)
    }
}

impl From<Contract> for StoredValue {
    fn from(value: Contract) -> Self {
        StoredValue::Contract(value)
    }
}

impl From<AddressableEntity> for StoredValue {
    fn from(value: AddressableEntity) -> StoredValue {
        StoredValue::AddressableEntity(value)
    }
}
impl From<Package> for StoredValue {
    fn from(value: Package) -> StoredValue {
        StoredValue::Package(value)
    }
}

impl From<Bid> for StoredValue {
    fn from(bid: Bid) -> StoredValue {
        StoredValue::Bid(Box::new(bid))
    }
}

impl From<BidKind> for StoredValue {
    fn from(bid_kind: BidKind) -> StoredValue {
        StoredValue::BidKind(bid_kind)
    }
}

impl From<ByteCode> for StoredValue {
    fn from(value: ByteCode) -> StoredValue {
        StoredValue::ByteCode(value)
    }
}

impl TryFrom<StoredValue> for CLValue {
    type Error = TypeMismatch;

    fn try_from(stored_value: StoredValue) -> Result<Self, Self::Error> {
        let type_name = stored_value.type_name();
        match stored_value {
            StoredValue::CLValue(cl_value) => Ok(cl_value),
            StoredValue::Package(contract_package) => Ok(CLValue::from_t(contract_package)
                .map_err(|_error| TypeMismatch::new("ContractPackage".to_string(), type_name))?),
            _ => Err(TypeMismatch::new("StoredValue".to_string(), type_name)),
        }
    }
}

impl TryFrom<StoredValue> for Account {
    type Error = TypeMismatch;

    fn try_from(stored_value: StoredValue) -> Result<Self, Self::Error> {
        match stored_value {
            StoredValue::Account(account) => Ok(account),
            _ => Err(TypeMismatch::new(
                "Account".to_string(),
                stored_value.type_name(),
            )),
        }
    }
}

impl TryFrom<StoredValue> for ContractWasm {
    type Error = TypeMismatch;

    fn try_from(stored_value: StoredValue) -> Result<Self, Self::Error> {
        match stored_value {
            StoredValue::ContractWasm(contract_wasm) => Ok(contract_wasm),
            _ => Err(TypeMismatch::new(
                "ContractWasm".to_string(),
                stored_value.type_name(),
            )),
        }
    }
}

impl TryFrom<StoredValue> for ByteCode {
    type Error = TypeMismatch;

    fn try_from(stored_value: StoredValue) -> Result<Self, Self::Error> {
        match stored_value {
            StoredValue::ByteCode(byte_code) => Ok(byte_code),
            _ => Err(TypeMismatch::new(
                "ByteCode".to_string(),
                stored_value.type_name(),
            )),
        }
    }
}

impl TryFrom<StoredValue> for ContractPackage {
    type Error = TypeMismatch;

    fn try_from(value: StoredValue) -> Result<Self, Self::Error> {
        match value {
            StoredValue::ContractPackage(contract_package) => Ok(contract_package),
            _ => Err(TypeMismatch::new(
                "ContractPackage".to_string(),
                value.type_name(),
            )),
        }
    }
}

impl TryFrom<StoredValue> for Contract {
    type Error = TypeMismatch;

    fn try_from(stored_value: StoredValue) -> Result<Self, Self::Error> {
        match stored_value {
            StoredValue::Contract(contract) => Ok(contract),
            _ => Err(TypeMismatch::new(
                "Contract".to_string(),
                stored_value.type_name(),
            )),
        }
    }
}

impl TryFrom<StoredValue> for Package {
    type Error = TypeMismatch;

    fn try_from(stored_value: StoredValue) -> Result<Self, Self::Error> {
        match stored_value {
            StoredValue::Package(contract_package) => Ok(contract_package),
            _ => Err(TypeMismatch::new(
                "ContractPackage".to_string(),
                stored_value.type_name(),
            )),
        }
    }
}

impl TryFrom<StoredValue> for AddressableEntity {
    type Error = TypeMismatch;

    fn try_from(stored_value: StoredValue) -> Result<Self, Self::Error> {
        match stored_value {
            StoredValue::AddressableEntity(contract) => Ok(contract),
            _ => Err(TypeMismatch::new(
                "AddressableEntity".to_string(),
                stored_value.type_name(),
            )),
        }
    }
}

impl TryFrom<StoredValue> for TransferV1 {
    type Error = TypeMismatch;

    fn try_from(value: StoredValue) -> Result<Self, Self::Error> {
        match value {
            StoredValue::LegacyTransfer(transfer_v1) => Ok(transfer_v1),
            _ => Err(TypeMismatch::new(
                "LegacyTransfer".to_string(),
                value.type_name(),
            )),
        }
    }
}

impl TryFrom<StoredValue> for DeployInfo {
    type Error = TypeMismatch;

    fn try_from(value: StoredValue) -> Result<Self, Self::Error> {
        match value {
            StoredValue::DeployInfo(deploy_info) => Ok(deploy_info),
            _ => Err(TypeMismatch::new(
                "DeployInfo".to_string(),
                value.type_name(),
            )),
        }
    }
}

impl TryFrom<StoredValue> for EraInfo {
    type Error = TypeMismatch;

    fn try_from(value: StoredValue) -> Result<Self, Self::Error> {
        match value {
            StoredValue::EraInfo(era_info) => Ok(era_info),
            _ => Err(TypeMismatch::new("EraInfo".to_string(), value.type_name())),
        }
    }
}

impl TryFrom<StoredValue> for Bid {
    type Error = TypeMismatch;

    fn try_from(value: StoredValue) -> Result<Self, Self::Error> {
        match value {
            StoredValue::Bid(bid) => Ok(*bid),
            _ => Err(TypeMismatch::new("Bid".to_string(), value.type_name())),
        }
    }
}

impl TryFrom<StoredValue> for BidKind {
    type Error = TypeMismatch;

    fn try_from(value: StoredValue) -> Result<Self, Self::Error> {
        match value {
            StoredValue::BidKind(bid_kind) => Ok(bid_kind),
            _ => Err(TypeMismatch::new("BidKind".to_string(), value.type_name())),
        }
    }
}

impl TryFrom<StoredValue> for NamedKeyValue {
    type Error = TypeMismatch;

    fn try_from(value: StoredValue) -> Result<Self, Self::Error> {
        match value {
            StoredValue::NamedKey(named_key_value) => Ok(named_key_value),
            _ => Err(TypeMismatch::new(
                "NamedKeyValue".to_string(),
                value.type_name(),
            )),
        }
    }
}

impl ToBytes for StoredValue {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut buffer = bytesrepr::allocate_buffer(self)?;
        self.write_bytes(&mut buffer)?;
        Ok(buffer)
    }

    fn serialized_length(&self) -> usize {
        U8_SERIALIZED_LENGTH
            + match self {
                StoredValue::CLValue(cl_value) => cl_value.serialized_length(),
                StoredValue::Account(account) => account.serialized_length(),
                StoredValue::ContractWasm(contract_wasm) => contract_wasm.serialized_length(),
                StoredValue::Contract(contract_header) => contract_header.serialized_length(),
                StoredValue::ContractPackage(contract_package) => {
                    contract_package.serialized_length()
                }
                StoredValue::LegacyTransfer(transfer_v1) => transfer_v1.serialized_length(),
                StoredValue::DeployInfo(deploy_info) => deploy_info.serialized_length(),
                StoredValue::EraInfo(era_info) => era_info.serialized_length(),
                StoredValue::Bid(bid) => bid.serialized_length(),
                StoredValue::Withdraw(withdraw_purses) => withdraw_purses.serialized_length(),
                StoredValue::Unbonding(unbonding_purses) => unbonding_purses.serialized_length(),
                StoredValue::AddressableEntity(entity) => entity.serialized_length(),
                StoredValue::BidKind(bid_kind) => bid_kind.serialized_length(),
                StoredValue::Package(package) => package.serialized_length(),
                StoredValue::ByteCode(byte_code) => byte_code.serialized_length(),
                StoredValue::MessageTopic(message_topic_summary) => {
                    message_topic_summary.serialized_length()
                }
                StoredValue::Message(message_digest) => message_digest.serialized_length(),
                StoredValue::NamedKey(named_key_value) => named_key_value.serialized_length(),
                StoredValue::Reservation(reservation_kind) => reservation_kind.serialized_length(),
            }
    }

    fn write_bytes(&self, writer: &mut Vec<u8>) -> Result<(), Error> {
        writer.push(self.tag() as u8);
        match self {
            StoredValue::CLValue(cl_value) => cl_value.write_bytes(writer),
            StoredValue::Account(account) => account.write_bytes(writer),
            StoredValue::ContractWasm(contract_wasm) => contract_wasm.write_bytes(writer),
            StoredValue::Contract(contract_header) => contract_header.write_bytes(writer),
            StoredValue::ContractPackage(contract_package) => contract_package.write_bytes(writer),
            StoredValue::LegacyTransfer(transfer_v1) => transfer_v1.write_bytes(writer),
            StoredValue::DeployInfo(deploy_info) => deploy_info.write_bytes(writer),
            StoredValue::EraInfo(era_info) => era_info.write_bytes(writer),
            StoredValue::Bid(bid) => bid.write_bytes(writer),
            StoredValue::Withdraw(unbonding_purses) => unbonding_purses.write_bytes(writer),
            StoredValue::Unbonding(unbonding_purses) => unbonding_purses.write_bytes(writer),
            StoredValue::AddressableEntity(entity) => entity.write_bytes(writer),
            StoredValue::BidKind(bid_kind) => bid_kind.write_bytes(writer),
            StoredValue::Package(package) => package.write_bytes(writer),
            StoredValue::ByteCode(byte_code) => byte_code.write_bytes(writer),
            StoredValue::MessageTopic(message_topic_summary) => {
                message_topic_summary.write_bytes(writer)
            }
            StoredValue::Message(message_digest) => message_digest.write_bytes(writer),
            StoredValue::NamedKey(named_key_value) => named_key_value.write_bytes(writer),
            StoredValue::Reservation(reservation_kind) => reservation_kind.write_bytes(writer),
        }
    }
}

impl FromBytes for StoredValue {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let (tag, remainder) = u8::from_bytes(bytes)?;
        match tag {
            tag if tag == Tag::CLValue as u8 => CLValue::from_bytes(remainder)
                .map(|(cl_value, remainder)| (StoredValue::CLValue(cl_value), remainder)),
            tag if tag == Tag::Account as u8 => Account::from_bytes(remainder)
                .map(|(account, remainder)| (StoredValue::Account(account), remainder)),
            tag if tag == Tag::ContractWasm as u8 => {
                ContractWasm::from_bytes(remainder).map(|(contract_wasm, remainder)| {
                    (StoredValue::ContractWasm(contract_wasm), remainder)
                })
            }
            tag if tag == Tag::ContractPackage as u8 => {
                ContractPackage::from_bytes(remainder).map(|(contract_package, remainder)| {
                    (StoredValue::ContractPackage(contract_package), remainder)
                })
            }
            tag if tag == Tag::Contract as u8 => Contract::from_bytes(remainder)
                .map(|(contract, remainder)| (StoredValue::Contract(contract), remainder)),
            tag if tag == Tag::LegacyTransfer as u8 => {
                TransferV1::from_bytes(remainder).map(|(transfer_v1, remainder)| {
                    (StoredValue::LegacyTransfer(transfer_v1), remainder)
                })
            }
            tag if tag == Tag::DeployInfo as u8 => DeployInfo::from_bytes(remainder)
                .map(|(deploy_info, remainder)| (StoredValue::DeployInfo(deploy_info), remainder)),
            tag if tag == Tag::EraInfo as u8 => EraInfo::from_bytes(remainder)
                .map(|(deploy_info, remainder)| (StoredValue::EraInfo(deploy_info), remainder)),
            tag if tag == Tag::Bid as u8 => Bid::from_bytes(remainder)
                .map(|(bid, remainder)| (StoredValue::Bid(Box::new(bid)), remainder)),
            tag if tag == Tag::BidKind as u8 => BidKind::from_bytes(remainder)
                .map(|(bid_kind, remainder)| (StoredValue::BidKind(bid_kind), remainder)),
            tag if tag == Tag::Withdraw as u8 => {
                Vec::<WithdrawPurse>::from_bytes(remainder).map(|(withdraw_purses, remainder)| {
                    (StoredValue::Withdraw(withdraw_purses), remainder)
                })
            }
            tag if tag == Tag::Unbonding as u8 => {
                Vec::<UnbondingPurse>::from_bytes(remainder).map(|(unbonding_purses, remainder)| {
                    (StoredValue::Unbonding(unbonding_purses), remainder)
                })
            }
            tag if tag == Tag::AddressableEntity as u8 => AddressableEntity::from_bytes(remainder)
                .map(|(entity, remainder)| (StoredValue::AddressableEntity(entity), remainder)),
            tag if tag == Tag::Package as u8 => Package::from_bytes(remainder)
                .map(|(package, remainder)| (StoredValue::Package(package), remainder)),
            tag if tag == Tag::ByteCode as u8 => ByteCode::from_bytes(remainder)
                .map(|(byte_code, remainder)| (StoredValue::ByteCode(byte_code), remainder)),
            tag if tag == Tag::MessageTopic as u8 => MessageTopicSummary::from_bytes(remainder)
                .map(|(message_summary, remainder)| {
                    (StoredValue::MessageTopic(message_summary), remainder)
                }),
            tag if tag == Tag::Message as u8 => MessageChecksum::from_bytes(remainder)
                .map(|(checksum, remainder)| (StoredValue::Message(checksum), remainder)),
            tag if tag == Tag::NamedKey as u8 => {
                NamedKeyValue::from_bytes(remainder).map(|(named_key_value, remainder)| {
                    (StoredValue::NamedKey(named_key_value), remainder)
                })
            }
            _ => Err(Error::Formatting),
        }
    }
}

mod serde_helpers {
    use core::fmt;

    use super::*;

    pub struct InvalidHumanReadableDeser(String);

    impl fmt::Display for InvalidHumanReadableDeser {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "Couldn't deserialize StoredValue. Underlying error: {}",
                self.0
            )
        }
    }

    #[derive(Serialize)]
    pub(super) enum HumanReadableSerHelper<'a> {
        CLValue(&'a CLValue),
        Account(&'a Account),
        ContractWasm(&'a ContractWasm),
        Contract(&'a Contract),
        ContractPackage(&'a ContractPackage),
        LegacyTransfer(&'a TransferV1),
        DeployInfo(&'a DeployInfo),
        EraInfo(&'a EraInfo),
        Bid(&'a Bid),
        Withdraw(&'a Vec<WithdrawPurse>),
        Unbonding(&'a Vec<UnbondingPurse>),
        AddressableEntity(&'a AddressableEntity),
        BidKind(&'a BidKind),
        Package(&'a Package),
        ByteCode(&'a ByteCode),
        MessageTopic(&'a MessageTopicSummary),
        Message(&'a MessageChecksum),
        NamedKey(&'a NamedKeyValue),
        Reservation(&'a ReservationKind),
    }

    #[derive(Deserialize)]
    pub(super) enum HumanReadableDeserHelper {
        CLValue(CLValue),
        Account(Account),
        ContractWasm(ContractWasm),
        Contract(Contract),
        ContractPackage(ContractPackage),
        LegacyTransfer(TransferV1),
        DeployInfo(DeployInfo),
        EraInfo(EraInfo),
        Bid(Box<Bid>),
        Withdraw(Vec<WithdrawPurse>),
        Unbonding(Vec<UnbondingPurse>),
        AddressableEntity(AddressableEntity),
        BidKind(BidKind),
        Package(Package),
        ByteCode(ByteCode),
        MessageTopic(MessageTopicSummary),
        Message(MessageChecksum),
        NamedKey(NamedKeyValue),
    }

    impl<'a> From<&'a StoredValue> for HumanReadableSerHelper<'a> {
        fn from(stored_value: &'a StoredValue) -> Self {
            match stored_value {
                StoredValue::CLValue(payload) => HumanReadableSerHelper::CLValue(payload),
                StoredValue::Account(payload) => HumanReadableSerHelper::Account(payload),
                StoredValue::ContractWasm(payload) => HumanReadableSerHelper::ContractWasm(payload),
                StoredValue::Contract(payload) => HumanReadableSerHelper::Contract(payload),
                StoredValue::ContractPackage(payload) => {
                    HumanReadableSerHelper::ContractPackage(payload)
                }
                StoredValue::LegacyTransfer(payload) => {
                    HumanReadableSerHelper::LegacyTransfer(payload)
                }
                StoredValue::DeployInfo(payload) => HumanReadableSerHelper::DeployInfo(payload),
                StoredValue::EraInfo(payload) => HumanReadableSerHelper::EraInfo(payload),
                StoredValue::Bid(payload) => HumanReadableSerHelper::Bid(payload),
                StoredValue::Withdraw(payload) => HumanReadableSerHelper::Withdraw(payload),
                StoredValue::Unbonding(payload) => HumanReadableSerHelper::Unbonding(payload),
                StoredValue::AddressableEntity(payload) => {
                    HumanReadableSerHelper::AddressableEntity(payload)
                }
                StoredValue::BidKind(payload) => HumanReadableSerHelper::BidKind(payload),
                StoredValue::Package(payload) => HumanReadableSerHelper::Package(payload),
                StoredValue::ByteCode(payload) => HumanReadableSerHelper::ByteCode(payload),
                StoredValue::MessageTopic(message_topic_summary) => {
                    HumanReadableSerHelper::MessageTopic(message_topic_summary)
                }
                StoredValue::Message(message_digest) => {
                    HumanReadableSerHelper::Message(message_digest)
                }
                StoredValue::NamedKey(payload) => HumanReadableSerHelper::NamedKey(payload),
                StoredValue::Reservation(payload) => HumanReadableSerHelper::Reservation(payload),
            }
        }
    }

    impl From<HumanReadableDeserHelper> for StoredValue {
        fn from(helper: HumanReadableDeserHelper) -> Self {
            match helper {
                HumanReadableDeserHelper::CLValue(payload) => StoredValue::CLValue(payload),
                HumanReadableDeserHelper::Account(payload) => StoredValue::Account(payload),
                HumanReadableDeserHelper::ContractWasm(payload) => {
                    StoredValue::ContractWasm(payload)
                }
                HumanReadableDeserHelper::Contract(payload) => StoredValue::Contract(payload),
                HumanReadableDeserHelper::ContractPackage(payload) => {
                    StoredValue::ContractPackage(payload)
                }
                HumanReadableDeserHelper::LegacyTransfer(payload) => {
                    StoredValue::LegacyTransfer(payload)
                }
                HumanReadableDeserHelper::DeployInfo(payload) => StoredValue::DeployInfo(payload),
                HumanReadableDeserHelper::EraInfo(payload) => StoredValue::EraInfo(payload),
                HumanReadableDeserHelper::Bid(bid) => StoredValue::Bid(bid),
                HumanReadableDeserHelper::Withdraw(payload) => StoredValue::Withdraw(payload),
                HumanReadableDeserHelper::Unbonding(payload) => StoredValue::Unbonding(payload),
                HumanReadableDeserHelper::AddressableEntity(payload) => {
                    StoredValue::AddressableEntity(payload)
                }
                HumanReadableDeserHelper::BidKind(payload) => StoredValue::BidKind(payload),
                HumanReadableDeserHelper::ByteCode(payload) => StoredValue::ByteCode(payload),
                HumanReadableDeserHelper::Package(payload) => StoredValue::Package(payload),
                HumanReadableDeserHelper::MessageTopic(message_topic_summary) => {
                    StoredValue::MessageTopic(message_topic_summary)
                }
                HumanReadableDeserHelper::Message(message_digest) => {
                    StoredValue::Message(message_digest)
                }
                HumanReadableDeserHelper::NamedKey(payload) => StoredValue::NamedKey(payload),
            }
        }
    }
}

impl Serialize for StoredValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serde_helpers::HumanReadableSerHelper::from(self).serialize(serializer)
        } else {
            let bytes = self
                .to_bytes()
                .map_err(|error| ser::Error::custom(format!("{:?}", error)))?;
            ByteBuf::from(bytes).serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for StoredValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            let json_helper = serde_helpers::HumanReadableDeserHelper::deserialize(deserializer)?;
            Ok(StoredValue::from(json_helper))
        } else {
            let bytes = ByteBuf::deserialize(deserializer)?.into_vec();
            bytesrepr::deserialize::<StoredValue>(bytes)
                .map_err(|error| de::Error::custom(format!("{:?}", error)))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{bytesrepr, gens, StoredValue};
    use proptest::proptest;
    use serde_json::Value;
    const STORED_VALUE_CONTRACT_PACKAGE_RAW: &str = r#"
    {
        "ContractPackage": {
          "access_key": "uref-024d69e50a458f337817d3d11ba95bdbdd6258ba8f2dc980644c9efdbd64945d-007",
          "versions": [
            {
              "protocol_version_major": 1,
              "contract_version": 1,
              "contract_hash": "contract-1b301b49505ec5eaec1787686c54818bd60836b9301cce3f5c0237560e5a4bfd"
            }
          ],
          "disabled_versions": [],
          "groups": [],
          "lock_status": "Unlocked"
        }
    }"#;
    const INCORRECT_STORED_VALUE_CONTRACT_PACKAGE_RAW: &str = r#"
    {
        "ContractPackage": {
          "access_key": "uref-024d69e50a458f337817d3d11ba95bdbdd6258ba8f2dc980644c9efdbd64945d-007",
          "versions": [
            {
              "protocol_version_major": 1,
              "contract_version": 1,
              "contract_hash": "contract-1b301b49505ec5eaec1787686c54818bd60836b9301cce3f5c0237560e5a4bfd"
            },
            {
              "protocol_version_major": 1,
              "contract_version": 1,
              "contract_hash": "contract-1b301b49505ec5eaec1787686c54818bd60836b9301cce3f5c0237560e5a4bfe"
            }
          ],
          "disabled_versions": [],
          "groups": [],
          "lock_status": "Unlocked"
        }
    }
    "#;

    #[test]
    fn contract_package_stored_value_serializes_versions_to_flat_array() {
        let value_from_raw_json =
            serde_json::from_str::<Value>(STORED_VALUE_CONTRACT_PACKAGE_RAW).unwrap();
        let deserialized =
            serde_json::from_str::<StoredValue>(STORED_VALUE_CONTRACT_PACKAGE_RAW).unwrap();
        let roundtrip_value = serde_json::to_value(&deserialized).unwrap();
        assert_eq!(value_from_raw_json, roundtrip_value);
    }

    #[test]
    fn contract_package_stored_value_should_fail_on_duplicate_keys() {
        let deserialization_res =
            serde_json::from_str::<StoredValue>(INCORRECT_STORED_VALUE_CONTRACT_PACKAGE_RAW);
        assert!(deserialization_res.is_err());
        assert!(deserialization_res
            .unwrap_err()
            .to_string()
            .contains("duplicate contract version: ContractVersionKey(1, 1)"));
    }

    proptest! {

        #[test]
        fn json_serialization_roundtrip(v in gens::stored_value_arb()) {
            let json_str = serde_json::to_string(&v).unwrap();
            let deserialized = serde_json::from_str::<StoredValue>(&json_str).unwrap();
            assert_eq!(v, deserialized);
        }

        #[test]
        fn serialization_roundtrip(v in gens::stored_value_arb()) {
            bytesrepr::test_serialization_roundtrip(&v);
        }
    }
}
