// SPDX-License-Identifier: UNLICENSED
pragma solidity >=0.4.24;

contract Store {
    uint256 public slot0;
    uint256 public slot1;
    uint256 public slot2;

    constructor() {
        slot0 = 2;
        slot1 = 3;
        slot2 = type(uint256).max;
    }

    function set(uint256 key, uint256 val) external {
        assembly {
            sstore(key, val)
        }
    }

    function kill() external {
        selfdestruct(payable(msg.sender));
    }
}

contract Factory {
    Store public last;
    event Deploy(address dst);
    function deploy(bytes32 salt) external returns (Store) {
        last = new Store{salt: salt}();
        return last;
    }
}
