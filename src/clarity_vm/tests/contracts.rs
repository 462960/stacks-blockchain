// Copyright (C) 2013-2020 Blockstack PBC, a public benefit corporation
// Copyright (C) 2020-2022 Stacks Open Internet Foundation
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use stacks_common::types::chainstate::BlockHeaderHash;
use stacks_common::types::chainstate::StacksBlockId;
use stacks_common::types::chainstate::BurnchainHeaderHash;

#[cfg(any(test, feature = "testing"))]
use rstest::rstest;
#[cfg(any(test, feature = "testing"))]
use rstest_reuse::{self, *};

use crate::chainstate::burn::BlockSnapshot;
use clarity::vm::ast;
use clarity::vm::ast::errors::ParseErrors;
use clarity::vm::contexts::{Environment, GlobalContext, OwnedEnvironment};
use clarity::vm::contracts::Contract;
use clarity::vm::costs::ExecutionCost;
use clarity::vm::database::ClarityDatabase;
use clarity::vm::errors::{CheckErrors, Error, RuntimeErrorType};
use clarity::vm::execute as vm_execute;
use clarity::vm::representations::SymbolicExpression;
use clarity::vm::tests::{
    execute, is_committed, is_err_code_i128 as is_err_code, symbols_from_values,
    with_memory_environment, TEST_BURN_STATE_DB, TEST_HEADER_DB, BurnStateDB,
};
use clarity::vm::types::{
    OptionalData, PrincipalData, QualifiedContractIdentifier, ResponseData, StandardPrincipalData,
    TypeSignature, Value,
};
use clarity::vm::ClarityVersion;
use stacks_common::util::hash::hex_bytes;
use stacks_common::types::chainstate::{ConsensusHash, SortitionId};
use stacks_common::types::StacksEpoch;

use clarity::vm::Value::Sequence;
use clarity::vm::types::SequenceData::Buffer;
use clarity::vm::types::BuffData;

use clarity::vm::database::MemoryBackingStore;

use crate::chainstate::stacks::boot::contract_tests::{ClarityTestSim, test_sim_height_to_hash};

#[test]
// Here, we set up a basic test to see if we can recover a path from the ClarityTestSim.
fn test_get_burn_block_info_eval() {
    let mut sim = ClarityTestSim::new();
    sim.epoch_bounds = vec![0, 1, 2];

    // Advance at least one block because 'get-burn-block-info' only works after the first block.
    sim.execute_next_block(|_env| {});
    // Advance another block so we get to Stacks 2.05. 
    sim.execute_next_block(|_env| {});
    // Advance another block so we get to Stacks 2.1. 
    sim.execute_next_block(|_env| {});
    sim.execute_next_block(|owned_env| {
        let contract_identifier = QualifiedContractIdentifier::local("test-contract").unwrap();
        let contract =
            "(define-private (test-func (height uint)) (get-burn-block-info? header-hash height))";
        owned_env
            .initialize_contract(contract_identifier.clone(), contract, None)
            .unwrap();

        // This relies on `TestSimBurnStateDB::get_burn_header_hash'
        assert_eq!(
            Value::Optional(OptionalData {
                data: Some(Box::new(Sequence(Buffer(BuffData {
                    data: test_sim_height_to_hash(0, 0).to_vec()
                }))))
            }),
            owned_env
                .eval_read_only(&contract_identifier, "(test-func u0)")
                .unwrap()
                .0
        );
        assert_eq!(
            Value::Optional(OptionalData {
                data: Some(Box::new(Sequence(Buffer(BuffData {
                    data: test_sim_height_to_hash(1, 0).to_vec()
                }))))
            }),
            owned_env
                .eval_read_only(&contract_identifier, "(test-func u1)")
                .unwrap()
                .0
        );
        assert_eq!(
            Value::Optional(OptionalData {
                data: Some(Box::new(Sequence(Buffer(BuffData {
                    data: test_sim_height_to_hash(2, 0).to_vec()
                }))))
            }),
            owned_env
                .eval_read_only(&contract_identifier, "(test-func u2)")
                .unwrap()
                .0
        );
    });
}
