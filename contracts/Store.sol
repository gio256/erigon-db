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

    function inc() external {
        slot0++;
        slot1++;
        slot2++;
    }

    fallback() external {
        slot1--;
    }
}
