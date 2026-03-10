// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

interface IRegistry {
    function register(address who) external;
}

library ArrayLib {
    function sum(uint256[] memory values) internal pure returns (uint256 total) {
        for (uint256 i = 0; i < values.length; i++) {
            total += values[i];
        }
    }
}

abstract contract BaseScriptTarget {
    constructor(address admin) {}
}

contract ComplexDeployment is BaseScriptTarget {
    struct Config {
        address owner;
        uint256[] limits;
    }

    error BadLabel(string reason);

    constructor(
        Config memory config,
        function(address, uint256[] memory) external returns (bytes32) callback,
        string memory label,
        bytes32 salt
    )
        BaseScriptTarget(config.owner)
        payable
    {
        string memory marker = "constructor(string should not confuse parser)";
        if (bytes(label).length == 0) revert BadLabel(marker);
        assembly {
            let slot := mload(0x40)
            mstore(slot, salt)
        }
    }
}
