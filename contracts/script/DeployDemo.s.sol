// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "forge-std/console2.sol";
import {MiniAMM} from "../src/MiniAMM.sol";
import {MockERC20} from "../src/MockERC20.sol";

/// @notice Deploys the classroom AMM demo and seeds it with toy liquidity.
///         Works on Anvil or Sepolia. Use test keys and test ETH only.
contract DeployDemo is Script {
    uint256 internal constant UNIT = 1e18;

    function run() external {
        uint256 deployerPk = vm.envUint("PRIVATE_KEY");
        uint256 attackerPk = vm.envOr("ATTACKER_PRIVATE_KEY", deployerPk);
        uint256 victimPk = vm.envOr("VICTIM_PRIVATE_KEY", deployerPk);

        address deployer = vm.addr(deployerPk);
        address attacker = vm.addr(attackerPk);
        address victim = vm.addr(victimPk);

        uint256 liquidityWad = vm.envOr("INITIAL_LIQUIDITY_WAD", uint256(100_000 * UNIT));
        uint256 traderFundingWad = vm.envOr("TRADER_FUNDING_WAD", uint256(10_000 * UNIT));
        uint16 feeBps = uint16(vm.envOr("FEE_BPS", uint256(30)));

        vm.startBroadcast(deployerPk);

        MockERC20 tokenX = new MockERC20("Demo Token X", "dX");
        MockERC20 tokenY = new MockERC20("Demo Token Y", "dY");
        MiniAMM pool = new MiniAMM(address(tokenX), address(tokenY), feeBps);

        tokenX.mint(deployer, liquidityWad);
        tokenY.mint(deployer, liquidityWad);
        tokenX.approve(address(pool), type(uint256).max);
        tokenY.approve(address(pool), type(uint256).max);
        pool.addLiquidity(liquidityWad, liquidityWad);

        tokenX.mint(attacker, traderFundingWad);
        tokenX.mint(victim, traderFundingWad);

        vm.stopBroadcast();

        console2.log("deployer", deployer);
        console2.log("attacker", attacker);
        console2.log("victim", victim);
        console2.log("TOKEN_X", address(tokenX));
        console2.log("TOKEN_Y", address(tokenY));
        console2.log("AMM_ADDRESS", address(pool));
        console2.log("feeBps", feeBps);
        console2.log("initial liquidity per side (wad)", liquidityWad);
        console2.log("trader X funding (wad)", traderFundingWad);
    }
}
