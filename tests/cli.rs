use assert_cmd::Command;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;
use predicates::str::contains;
use std::error::Error;

fn write_contract(
    dir: &assert_fs::fixture::ChildPath,
    name: &str,
    source: &str,
) -> Result<(), Box<dyn Error>> {
    let file = dir.child(name);
    file.write_str(source)?;
    Ok(())
}

#[test]
fn lists_discovered_contracts() -> Result<(), Box<dyn Error>> {
    let project = TempDir::new()?;
    let src_dir = project.child("src");
    src_dir.create_dir_all()?;

    write_contract(
        &src_dir,
        "Vault.sol",
        r#"// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract Vault {
    constructor(address owner, uint256 timelock) {}
}

contract Helper {
    constructor() {}
}
"#,
    )?;

    let mut cmd = Command::cargo_bin("forge-scriptgen")?;
    cmd.current_dir(project.path());
    cmd.arg("--list");

    cmd.assert()
        .success()
        .stdout(contains("Vault (src/Vault.sol)"))
        .stdout(contains("constructor(address owner, uint256 timelock)"))
        .stdout(contains("Helper (src/Vault.sol)"))
        .stdout(contains("constructor(): no parameters"));

    Ok(())
}

#[test]
fn generates_script_with_args() -> Result<(), Box<dyn Error>> {
    let project = TempDir::new()?;
    let src_dir = project.child("src");
    src_dir.create_dir_all()?;

    write_contract(
        &src_dir,
        "Counter.sol",
        r#"// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

contract Counter {
    uint256 public value;

    constructor(uint256 initialValue, string memory label) {
        value = initialValue;
    }
}
"#,
    )?;

    let mut cmd = Command::cargo_bin("forge-scriptgen")?;
    cmd.current_dir(project.path());
    cmd.args([
        "--args",
        r#"[42,"Counter One"]"#,
        "--private-key",
        "0x0123",
        "Counter",
    ]);

    cmd.assert()
        .success()
        .stdout(contains("Generated script at script/Counter.s.sol"));

    let script_path = project.child("script/Counter.s.sol");
    script_path.assert(predicate::path::exists());
    let contents = std::fs::read_to_string(script_path.path())?;

    assert!(contents.contains("pragma solidity ^0.8.18"));
    assert!(contents.contains("import \"../src/Counter.sol\";"));
    assert!(contents.contains("uint256 deployerPrivateKey = 0x0123;"));
    assert!(contents.contains("new Counter(42, \"Counter One\");"));

    Ok(())
}

#[test]
fn generates_script_for_complex_constructor_signatures() -> Result<(), Box<dyn Error>> {
    let project = TempDir::new()?;
    let src_dir = project.child("src");
    src_dir.create_dir_all()?;

    write_contract(
        &src_dir,
        "AdvancedCounter.sol",
        r#"// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

abstract contract BaseDeployer {
    constructor(address admin) {}
}

contract AdvancedCounter is BaseDeployer {
    struct Config {
        address owner;
        uint256[] limits;
    }

    constructor(
        Config memory config,
        function(address, uint256[] memory) external returns (bytes32) callback,
        string memory label
    ) BaseDeployer(config.owner) payable {}
}
"#,
    )?;

    let mut list_cmd = Command::cargo_bin("forge-scriptgen")?;
    list_cmd.current_dir(project.path());
    list_cmd.args(["--parser", "string-walker", "--list"]);
    list_cmd
        .assert()
        .success()
        .stdout(contains("AdvancedCounter (src/AdvancedCounter.sol)"))
        .stdout(contains("Config memory config"))
        .stdout(contains("callback"))
        .stdout(contains("string memory label"));

    let mut generate_cmd = Command::cargo_bin("forge-scriptgen")?;
    generate_cmd.current_dir(project.path());
    generate_cmd.args([
        "--parser",
        "string-walker",
        "--args",
        r#"[{"raw":"configLiteral"},{"raw":"callbackLiteral"},"primary"]"#,
        "--private-key",
        "0x9999",
        "AdvancedCounter",
    ]);

    generate_cmd
        .assert()
        .success()
        .stdout(contains("Generated script at script/AdvancedCounter.s.sol"));

    let script_path = project.child("script/AdvancedCounter.s.sol");
    script_path.assert(predicate::path::exists());
    let contents = std::fs::read_to_string(script_path.path())?;

    assert!(contents.contains("new AdvancedCounter(configLiteral, callbackLiteral, \"primary\");"));

    Ok(())
}
