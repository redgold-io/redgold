use crate::proto_serde::{ProtoHashable, ProtoSerde};
use crate::structs::{ErrorCode, Hash, StructMetadata};
use crate::{HashClear, RgResult, SafeOption};
use prost::Message;

pub trait WithMetadataHashableFields {
    fn struct_metadata_opt(&mut self) -> Option<&mut StructMetadata>;
    // fn struct_metadata(&self) -> Option<&StructMetadata>;
    fn struct_metadata_opt_ref(&self) -> Option<&StructMetadata>;
}

pub trait WithMetadataHashable {
    fn struct_metadata(&mut self) -> RgResult<&mut StructMetadata>;
    fn struct_metadata_err(&self) -> RgResult<&StructMetadata>;
    fn version(&self) -> RgResult<i32>;
    fn time(&self) -> RgResult<&i64>;
    fn hash_or(&self) -> Hash;
    fn hash_proto_bytes(&self) -> Vec<u8>;
    fn hash_hex(&self) -> String;
    fn with_hash(&mut self) -> &mut Self;
    fn set_hash(&mut self, hash: Hash) -> RgResult<()>;
}

impl<T> WithMetadataHashable for T
where
    Self: WithMetadataHashableFields + HashClear + Clone + Message + std::default::Default,
{
    fn struct_metadata(&mut self) -> RgResult<&mut StructMetadata> {
        let option = self.struct_metadata_opt();
        option.ok_or(crate::error_message(ErrorCode::MissingField, "struct_metadata"))
    }

    fn struct_metadata_err(&self) -> RgResult<&StructMetadata> {
        self.struct_metadata_opt_ref().ok_or(crate::error_message(ErrorCode::MissingField, "struct_metadata"))
    }

    fn version(&self) -> RgResult<i32> {
        Ok(self.struct_metadata_err()?.version)
    }

    fn time(&self) -> RgResult<&i64> {
        Ok(self.struct_metadata_opt_ref().safe_get()?.time.safe_get()?)
    }

    fn hash_or(&self) -> Hash {
        self.struct_metadata_opt_ref()
            .and_then(|s| s.hash.clone()) // TODO: Change to as_ref() to prevent clone?
            .unwrap_or(self.calculate_hash())
    }


    fn hash_proto_bytes(&self) -> Vec<u8> {
        self.hash_or().proto_serialize()
    }

    fn hash_hex(&self) -> String {
        self.hash_or().hex()
    }

    fn with_hash(&mut self) -> &mut T {
        let hash = self.calculate_hash();
        self.set_hash(hash).expect("set");
        self
    }

    fn set_hash(&mut self, hash: Hash) -> RgResult<()> {
        let met = self.struct_metadata()?;
        met.hash = Some(hash);
        Ok(())
    }
}
