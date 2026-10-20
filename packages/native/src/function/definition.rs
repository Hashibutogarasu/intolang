use errors::{Error, Errors, Result};
use parser::Record;
use types::MessageTag;

/// `NATIVE_FUNCTION_BEGIN`: the statements up to `NATIVE_HANDLER_END` form the
/// body of a function that is called at run time. Fields: the function name,
/// the type it returns (`Void` for none), then one `name:Type` per parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDefinition {
    pub name: String,
    pub return_type: String,
    pub params: Vec<(String, String)>,
}

impl FunctionDefinition {
    pub fn to_record(&self) -> Record {
        let mut record = Record::new(MessageTag::FunctionBegin)
            .field(&self.name)
            .field(&self.return_type);
        for (name, ty) in &self.params {
            record = record.field(format!("{name}:{ty}"));
        }
        record
    }
}

impl TryFrom<&Record> for FunctionDefinition {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let [name, return_type, params @ ..] = record.fields() else {
            return Err(Errors::malformed_record("NATIVE_FUNCTION_BEGIN").into());
        };
        let params = params
            .iter()
            .map(|field| {
                field
                    .split_once(':')
                    .map(|(name, ty)| (name.to_owned(), ty.to_owned()))
                    .ok_or_else(|| {
                        Errors::MalformedParameter {
                            field: field.clone(),
                        }
                        .into()
                    })
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            name: name.clone(),
            return_type: return_type.clone(),
            params,
        })
    }
}
