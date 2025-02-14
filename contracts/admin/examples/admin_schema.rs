use admin_contract::msg::{InstantiateMsg, MigrateMsg, SudoMsg};
use sdk::cosmwasm_schema::{export_schema, schema_for};

fn main() {
    let out_dir = schema::prep_out_dir().expect("The output directory should be valid");

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(MigrateMsg), &out_dir);
    export_schema(&schema_for!(SudoMsg), &out_dir);
}
