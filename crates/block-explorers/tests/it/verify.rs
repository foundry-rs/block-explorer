#![cfg(feature = "compilers-full")]

use crate::{run_with_client, run_with_client_v2};
use alloy_chains::Chain;
use foundry_block_explorers::verify::{CodeFormat, VerifyContract};
use foundry_compilers::{solc::SolcLanguage, Project, ProjectPathsConfig};
use serial_test::serial;
use std::path::Path;
use tokio::time::{sleep, Duration};
use tracing::trace;

#[tokio::test]
#[serial]
async fn test_can_flatten_and_verify_contract_single_file() {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-data/rewards"));
    let paths = ProjectPathsConfig::builder()
        .sources(root)
        .build()
        .expect("failed to resolve project paths");
    let project = Project::builder()
        .paths(paths)
        .build(Default::default())
        .expect("failed to build the project");

    let address = "0x7777777F279eba3d3Ad8F4E708545291A6fDBA8B".parse().unwrap();
    let compiler_version = "v0.8.17+commit.8df45f5f";
    let contract = project
        .paths
        .clone()
        .with_language::<SolcLanguage>()
        .flatten(&root.join("ProtocolRewards.sol"))
        .expect("failed to flatten contract");
    let contract_name = "ProtocolRewards.sol:ProtocolRewards".to_owned();
    let contract =
        VerifyContract::new(address, contract_name, contract, compiler_version.to_string())
            .code_format(CodeFormat::SingleFile)
            .optimization(true)
            .runs(500000);

    run_with_client(Chain::mainnet(), |client| async move {
        let resp = client
            .submit_contract_verification(&contract)
            .await
            .expect("failed to send the request");
        // `Error!` result means that request was malformatted
        assert_ne!(resp.result, "Error!", "{resp:?}");
        assert_ne!(resp.message, "NOTOK", "{resp:?}");
    })
    .await
}

#[tokio::test]
#[serial]
async fn can_verify_contract_json_multi_file() {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-data/rewards"));

    let paths = ProjectPathsConfig::builder()
        .sources(root)
        .build()
        .expect("failed to resolve project paths");

    let compiler_version = "v0.8.17+commit.8df45f5f";

    let mut project = Project::builder()
        .paths(paths)
        .build(Default::default())
        .expect("failed to build the project");

    // Remove the evmVersion from standard input build in the project.
    project.settings.solc.evm_version = None;

    // This is an unverified address so we can check the verify failure message.
    // That message ensured that the contract successfully compiled on the etherscan side.
    let address = "0x495f947276749Ce646f68AC8c248420045cb7b5e".parse().unwrap();
    let contract = project.clone().standard_json_input(&root.join("ProtocolRewards.sol"));
    let contract_name = "test-data/rewards/ProtocolRewards.sol:ProtocolRewards".to_owned();
    let contract_json_string =
        serde_json::to_string(&contract.expect("Contract did not compile to standard json"));

    let contract = VerifyContract::new(
        address,
        contract_name,
        contract_json_string.expect("Did not serialize json from standard json output"),
        compiler_version.to_string(),
    )
    .code_format(CodeFormat::StandardJsonInput)
    .optimization(true)
    .runs(500000);

    run_with_client(Chain::mainnet(), |client| async move {
        let resp = client
            .submit_contract_verification(&contract)
            .await
            .expect("failed to send the request");


        // The new API returns this error when a contract source is already verified.
        assert_eq!(resp.message, "OK", "{resp:?}");

        let result = &resp.result.clone();

        // Attempt 5 times
        for _ in 1..5 {
            sleep(Duration::from_millis(5000)).await;
            let check = client.check_contract_verification_status(result).await.expect("failed to send the check");
            if check.result == "Pending in queue" {
                continue;
            }
            assert_eq!(check.message, "NOTOK", "{check:?}");
            assert_eq!(check.result, "Fail - Unable to verify. Compiled contract deployment bytecode does NOT match the transaction deployment bytecode.");
            break;
        }
    })
    .await
}

#[tokio::test]
#[serial]
async fn can_verify_contract_json_multi_file_v2_base() {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-data/rewards"));

    let compiler_version = "v0.8.17+commit.8df45f5f";

    let paths = ProjectPathsConfig::builder()
        .sources(root)
        .build()
        .expect("failed to resolve project paths");

    let project = Project::builder()
        .paths(paths)
        .build(Default::default())
        .expect("failed to build the project");

    let address = "0x7777777F279eba3d3Ad8F4E708545291A6fDBA8B".parse().unwrap();
    let contract = project.clone().standard_json_input(&root.join("ProtocolRewards.sol"));
    let contract_name = "ProtocolRewards.sol:ProtocolRewards".to_owned();
    let contract_json_string =
        serde_json::to_string(&contract.expect("Contract did not compile to standard json"));
    let contract = VerifyContract::new(
        address,
        contract_name,
        contract_json_string.expect("Did not serialize json from standard json output"),
        compiler_version.to_string(),
    )
    .code_format(CodeFormat::SingleFile)
    .optimization(true)
    .runs(500000);

    run_with_client_v2(Chain::base_mainnet(), |client| async move {
        let resp = client
            .submit_contract_verification(&contract)
            .await
            .expect("failed to send the request");

        // The new API returns this error when a contract source is already verified.
        assert_eq!(resp.result, "Contract source code already verified");
        assert_eq!(resp.message, "NOTOK", "{resp:?}");
    })
    .await
}

#[tokio::test]
#[serial]
async fn can_verify_contract_json_multi_file_v2_fails_base() {
    let compiler_version = "v0.8.17+commit.8df45f5f";

    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-data/rewards"));

    let paths = ProjectPathsConfig::builder()
        .sources(root)
        .build()
        .expect("failed to resolve project paths");

    let mut project = Project::builder()
        .paths(paths)
        .build(Default::default())
        .expect("failed to build the project");

    // Remove the EVM version from the verification JSON to fix the verification on v2.
    project.settings.solc.evm_version = None;

    // This is an unknown address so we can check the verify failure message.
    // That message ensured that the contract successfully compiled on the etherscan side.
    let address = "0x7ffDf45de9023AdC014009470941bCD965bf31b9".parse().unwrap();

    let contract = project.clone().standard_json_input(&root.join("ProtocolRewards.sol"));
    let contract_name = "test-data/rewards/ProtocolRewards.sol:ProtocolRewards".to_owned();
    let contract_json_string =
        serde_json::to_string(&contract.expect("Contract did not compile to standard json"))
            .expect("Did not serialize JSON for verification");

    trace!("{:?}", contract_json_string.clone());

    let contract = VerifyContract::new(
        address,
        contract_name,
        contract_json_string,
        compiler_version.to_string(),
    )
    .code_format(CodeFormat::StandardJsonInput)
    .optimization(true)
    .runs(500000);

    run_with_client_v2(Chain::base_mainnet(), |client| async move {
        let resp = client
            .submit_contract_verification(&contract)
            .await
            .expect("failed to send the request");

        // The new API returns this error when a contract source is already verified.
        assert_eq!(resp.message, "OK", "{resp:?}");

        let result = &resp.result.clone();

        // Attempt 5 times
        for _ in 1..5 {
            sleep(Duration::from_millis(5000)).await;
            let check = client.check_contract_verification_status(result).await.expect("failed to send the check");
            if check.result == "Pending in queue" {
                continue;
            }
            assert_eq!(check.message, "NOTOK", "{check:?}");
            assert_eq!(check.result, "Fail - Unable to verify. Compiled contract deployment bytecode does NOT match the transaction deployment bytecode.");
            break;
        }
    })
    .await
}

#[tokio::test]
#[serial]
async fn can_verify_contract_sol_flatten_v2_fails_base() {
    let compiler_version = "v0.8.17+commit.8df45f5f";

    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-data/rewards"));

    let paths = ProjectPathsConfig::builder()
        .sources(root)
        .build()
        .expect("failed to resolve project paths");

    let project = Project::builder()
        .paths(paths)
        .build(Default::default())
        .expect("failed to build the project");

    // This is an unknown address so we can check the verify failure message.
    let address = "0x7ffDf45de9023AdC014009470941bCD965bf31b9".parse().unwrap();

    let contract_flattened = project
        .clone()
        .paths
        .with_language::<SolcLanguage>()
        .flatten(&root.join("ProtocolRewards.sol"))
        .expect("failed to flatten contract");

    let contract_name = "ProtocolRewards".to_owned();

    trace!("{:?}", contract_flattened.clone());

    let contract = VerifyContract::new(
        address,
        contract_name,
        contract_flattened,
        compiler_version.to_string(),
    )
    .code_format(CodeFormat::SingleFile)
    .optimization(true)
    .runs(500000);

    run_with_client_v2(Chain::base_mainnet(), |client| async move {
        let resp = client
            .submit_contract_verification(&contract)
            .await
            .expect("failed to send the request");

        // The new API returns this error when a contract source is already verified.
        assert_eq!(resp.message, "OK", "{resp:?}");

        let result = &resp.result.clone();

        // Attempt 5 times
        for _ in 1..5 {
            sleep(Duration::from_millis(5000)).await;
            let check = client.check_contract_verification_status(result).await.expect("failed to send the check");
            if check.result == "Pending in queue" {
                continue;
            }
            assert_eq!(check.message, "NOTOK", "{check:?}");
            assert_eq!(check.result, "Fail - Unable to verify. Compiled contract deployment bytecode does NOT match the transaction deployment bytecode.");
            break;
        }
    })
    .await
}
