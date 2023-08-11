// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;
contract HelloWorld {


    uint112 private reserve0 = 0;           // uses single storage slot, accessible via getReserves
    uint112 private reserve1 = 1;           // uses single storage slot, accessible via getReserves
    uint32  private blockTimestampLast = 2; // uses single storage slot, accessible via getReserves

    function sayHelloWorld() external view returns (uint32 test, uint32 a) {
//        return "Hello World";
        return (1, 2);
    }

    function sayHelloWorld2() public pure returns (string memory) {
        return "Hello World";
    }

    function getReserves() public view returns (uint112 _reserve0, uint112 _reserve1, uint32 _blockTimestampLast) {
        _reserve0 = uint112(1);
        _reserve1 = uint112(2);
        _blockTimestampLast = uint32(1);
    }

    function getReserves2() public view returns (uint112 _reserve0, uint112 _reserve1, uint32 _blockTimestampLast) {
        _reserve0 = reserve0;
        _reserve1 = reserve1;
        _blockTimestampLast = blockTimestampLast;
    }

    function something() public view returns (uint256 _test) {
        _test = uint256(12123123123123123);
    }

}