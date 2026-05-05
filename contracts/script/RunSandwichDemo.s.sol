// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "forge-std/console2.sol";
import {MiniAMM} from "../src/MiniAMM.sol";
import {MockERC20} from "../src/MockERC20.sol";

/// @notice Runs front-run -> victim swap -> back-run against a deployed
///         MiniAMM. This is a mechanism demo, not a real MEV searcher.
contract RunSandwichDemo is Script {
    uint256 internal constant UNIT = 1e18;

    MiniAMM internal pool;
    MockERC20 internal tokenX;
    MockERC20 internal tokenY;
    uint256 internal attackerPk;
    uint256 internal victimPk;
    address internal attacker;
    address internal victim;
    uint256 internal attackerIn;
    uint256 internal victimIn;
    uint256 internal slippageBps;

    function run() external {
        _loadConfig();

        uint256 honestQuote = pool.quoteXForY(victimIn);
        uint256 minOut = (honestQuote * (10_000 - slippageBps)) / 10_000;
        uint256 attackerXBefore = tokenX.balanceOf(attacker);

        console2.log("attacker", attacker);
        console2.log("victim", victim);
        console2.log("honest quote Y (wad)", honestQuote);
        console2.log("victim minOut Y (wad)", minOut);

        uint256 frontOut = _frontRun();
        uint256 victimOut = _victimSwap(minOut);
        uint256 backOut = _backRun(frontOut);

        _logFinal(attackerXBefore, honestQuote, victimOut, backOut);
    }

    function _loadConfig() internal {
        uint256 defaultPk = vm.envUint("PRIVATE_KEY");
        attackerPk = vm.envOr("ATTACKER_PRIVATE_KEY", defaultPk);
        victimPk = vm.envOr("VICTIM_PRIVATE_KEY", defaultPk);

        pool = MiniAMM(vm.envAddress("AMM_ADDRESS"));
        tokenX = MockERC20(vm.envAddress("TOKEN_X"));
        tokenY = MockERC20(vm.envAddress("TOKEN_Y"));

        attackerIn = vm.envOr(
            "ATTACKER_IN_WAD",
            uint256(507_044_774_798_056_237_000)
        );
        victimIn = vm.envOr("VICTIM_IN_WAD", uint256(1_000 * UNIT));
        slippageBps = vm.envOr("SLIPPAGE_BPS", uint256(100));

        attacker = vm.addr(attackerPk);
        victim = vm.addr(victimPk);
    }

    function _frontRun() internal returns (uint256 frontOut) {
        vm.startBroadcast(attackerPk);
        tokenX.approve(address(pool), type(uint256).max);
        tokenY.approve(address(pool), type(uint256).max);
        frontOut = pool.swapXForY(attackerIn, 0);
        vm.stopBroadcast();
        console2.log("attacker front-run in X (wad)", attackerIn);
        console2.log("attacker front-run out Y (wad)", frontOut);
    }

    function _victimSwap(uint256 minOut) internal returns (uint256 victimOut) {
        vm.startBroadcast(victimPk);
        tokenX.approve(address(pool), type(uint256).max);
        victimOut = pool.swapXForY(victimIn, minOut);
        vm.stopBroadcast();
        console2.log("victim actual out Y (wad)", victimOut);
    }

    function _backRun(uint256 frontOut) internal returns (uint256 backOut) {
        vm.startBroadcast(attackerPk);
        backOut = pool.swapYForX(frontOut, 0);
        vm.stopBroadcast();
        console2.log("attacker back-run out X (wad)", backOut);
    }

    function _logFinal(
        uint256 attackerXBefore,
        uint256 honestQuote,
        uint256 victimOut,
        uint256 /* backOut */
    ) internal view {
        uint256 attackerXAfter = tokenX.balanceOf(attacker);
        if (attackerXAfter >= attackerXBefore) {
            console2.log("attacker profit X (wad)", attackerXAfter - attackerXBefore);
        } else {
            console2.log("attacker loss X (wad)", attackerXBefore - attackerXAfter);
        }
        console2.log("victim extra loss Y (wad)", honestQuote - victimOut);
    }
}
