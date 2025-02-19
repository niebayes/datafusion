// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! [`ScalarUDFImpl`] definitions for array_empty function.

use crate::utils::make_scalar_function;
use arrow_array::{ArrayRef, BooleanArray, OffsetSizeTrait};
use arrow_schema::DataType;
use arrow_schema::DataType::{Boolean, FixedSizeList, LargeList, List};
use datafusion_common::cast::as_generic_list_array;
use datafusion_common::{exec_err, plan_err, Result};
use datafusion_expr::scalar_doc_sections::DOC_SECTION_ARRAY;
use datafusion_expr::{
    ColumnarValue, Documentation, ScalarUDFImpl, Signature, Volatility,
};
use std::any::Any;
use std::sync::{Arc, OnceLock};

make_udf_expr_and_func!(
    ArrayEmpty,
    array_empty,
    array,
    "returns true for an empty array or false for a non-empty array.",
    array_empty_udf
);

#[derive(Debug)]
pub(super) struct ArrayEmpty {
    signature: Signature,
    aliases: Vec<String>,
}
impl ArrayEmpty {
    pub fn new() -> Self {
        Self {
            signature: Signature::array(Volatility::Immutable),
            aliases: vec!["array_empty".to_string(), "list_empty".to_string()],
        }
    }
}

impl ScalarUDFImpl for ArrayEmpty {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn name(&self) -> &str {
        "empty"
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, arg_types: &[DataType]) -> Result<DataType> {
        Ok(match arg_types[0] {
            List(_) | LargeList(_) | FixedSizeList(_, _) => Boolean,
            _ => {
                return plan_err!("The array_empty function can only accept List/LargeList/FixedSizeList.");
            }
        })
    }

    fn invoke(&self, args: &[ColumnarValue]) -> Result<ColumnarValue> {
        make_scalar_function(array_empty_inner)(args)
    }

    fn aliases(&self) -> &[String] {
        &self.aliases
    }

    fn documentation(&self) -> Option<&Documentation> {
        Some(get_empty_doc())
    }
}

static DOCUMENTATION: OnceLock<Documentation> = OnceLock::new();

fn get_empty_doc() -> &'static Documentation {
    DOCUMENTATION.get_or_init(|| {
        Documentation::builder()
            .with_doc_section(DOC_SECTION_ARRAY)
            .with_description(
                "Returns 1 for an empty array or 0 for a non-empty array.",
            )
            .with_syntax_example("empty(array)")
            .with_sql_example(
                r#"```sql
> select empty([1]);
+------------------+
| empty(List([1])) |
+------------------+
| 0                |
+------------------+
```"#,
            )
            .with_argument(
                "array",
                "Array expression. Can be a constant, column, or function, and any combination of array operators.",
            )
            .build()
            .unwrap()
    })
}

/// Array_empty SQL function
pub fn array_empty_inner(args: &[ArrayRef]) -> Result<ArrayRef> {
    if args.len() != 1 {
        return exec_err!("array_empty expects one argument");
    }

    let array_type = args[0].data_type();
    match array_type {
        List(_) => general_array_empty::<i32>(&args[0]),
        LargeList(_) => general_array_empty::<i64>(&args[0]),
        _ => exec_err!("array_empty does not support type '{array_type:?}'."),
    }
}

fn general_array_empty<O: OffsetSizeTrait>(array: &ArrayRef) -> Result<ArrayRef> {
    let array = as_generic_list_array::<O>(array)?;

    let builder = array
        .iter()
        .map(|arr| arr.map(|arr| arr.is_empty()))
        .collect::<BooleanArray>();
    Ok(Arc::new(builder))
}
