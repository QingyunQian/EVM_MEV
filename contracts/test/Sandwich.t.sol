// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "forge-std/console2.sol";
import {MiniAMM} from "../src/MiniAMM.sol";
import {MockERC20} from "../src/MockERC20.sol";

/// @notice On-chain sanity check for the Rust simulator. We deploy a pool,
///         reproduce the reference scenario (100k/100k, 30 bps, victim=1000,
///         slippage=1%), execute front-run -> victim -> back-run, and assert
///         the attacker's net profit lands within 1% of the Rust reference
///         (~7.016 X), plus structural invariants (victim respected slippage,
///         attacker ended richer in X).
contract SandwichTest is Test {
    MiniAMM internal pool;
    MockERC20 internal X;
    MockERC20 internal Y;

    address internal attacker = address(0xA11CE);
    address internal victim = address(0xB0B);
    address internal lp = address(0xC0DE);

    uint256 internal constant UNIT = 1e18;

    // Reference values from the Rust simulator for v=1000, slippage=1%,
    // pool 100k/100k, fee=30bps.
    uint256 internal constant ATTACKER_A_WAD = 507_044_774_798_056_237_000; // 507.044775e18
    uint256 internal constant OVERSIZED_A_WAD = 2_000 * UNIT;
    uint256 internal constant RUST_PROFIT_WAD = 7_016_248_550_825_367_000; // 7.016249e18
    uint256 internal constant RUST_HONEST_OUT_WAD = 987_158_034_397_061_300_000; // 987.158034e18
    uint256 internal constant RUST_OVERSIZED_LOSS_WAD = 11_748_438_793_734_066_000; // 11.748439e18

    function setUp() public {
        X = new MockERC20("TokenX", "X");
        Y = new MockERC20("TokenY", "Y");
        pool = new MiniAMM(address(X), address(Y), 30);

        X.mint(lp, 100_000 * UNIT);
        Y.mint(lp, 100_000 * UNIT);
        vm.startPrank(lp);
        X.approve(address(pool), type(uint256).max);
        Y.approve(address(pool), type(uint256).max);
        pool.addLiquidity(100_000 * UNIT, 100_000 * UNIT);
        vm.stopPrank();

        X.mint(attacker, 10_000 * UNIT);
        X.mint(victim, 10_000 * UNIT);

        vm.prank(attacker);
        X.approve(address(pool), type(uint256).max);
        vm.prank(attacker);
        Y.approve(address(pool), type(uint256).max);
        vm.prank(victim);
        X.approve(address(pool), type(uint256).max);
    }

    /// Baseline: no sandwich. Victim should receive the honest CPMM quote,
    /// which matches the Rust formula to integer precision.
    function test_honest_victim_swap_matches_formula() public {
        uint256 quoted = pool.quoteXForY(1_000 * UNIT);
        vm.prank(victim);
        uint256 out = pool.swapXForY(1_000 * UNIT, 0);
        assertEq(out, quoted, "quote==swap out");
        _assertClose(out, RUST_HONEST_OUT_WAD, 1e12, "matches rust honest out");
    }

    /// Full sandwich. We use the attacker size the Rust optimizer produced,
    /// then compare attacker profit with Rust's reported value.
    function test_sandwich_profit_matches_rust() public {
        uint256 honestQuote = pool.quoteXForY(1_000 * UNIT);
        uint256 minOut = (honestQuote * 99) / 100;

        uint256 attackerXBefore = X.balanceOf(attacker);
        console2.log("honest quote Y (wad)", honestQuote);
        console2.log("victim minOut Y (wad)", minOut);

        vm.prank(attacker);
        uint256 frontOut = pool.swapXForY(ATTACKER_A_WAD, 0);
        assertGt(frontOut, 0, "front out > 0");
        console2.log("attacker front-run in X (wad)", ATTACKER_A_WAD);
        console2.log("attacker front-run out Y (wad)", frontOut);

        vm.prank(victim);
        uint256 victimOut = pool.swapXForY(1_000 * UNIT, minOut);
        assertGe(victimOut, minOut, "victim slippage honored");
        console2.log("victim actual out Y (wad)", victimOut);

        vm.prank(attacker);
        uint256 backOut = pool.swapYForX(frontOut, 0);
        console2.log("attacker back-run out X (wad)", backOut);

        uint256 attackerXAfter = X.balanceOf(attacker);
        int256 profit = int256(attackerXAfter) - int256(attackerXBefore);
        assertGt(profit, 0, "attacker profitable");

        uint256 uprofit = uint256(profit);
        console2.log("attacker profit X (wad)", uprofit);
        // Allow 1% deviation: on-chain math rounds at every swap; Rust uses f64.
        uint256 tol = RUST_PROFIT_WAD / 100;
        _assertClose(uprofit, RUST_PROFIT_WAD, tol, "profit within 1% of rust");

        // Victim's extra loss should be close to their 1% slippage tolerance.
        uint256 extraLoss = honestQuote - victimOut;
        uint256 maxLoss = honestQuote / 100;
        console2.log("victim extra loss Y (wad)", extraLoss);
        assertLe(extraLoss, maxLoss, "extra loss bounded by slippage");
        // At the Rust-optimal A, extra loss is essentially at the slippage cap.
        assertGt(extraLoss * 100, maxLoss * 99, "extra loss near slippage cap");

        // Unused but kept for clarity in test output:
        backOut;
    }

    /// Oversized front-run. The victim should revert because the attack moved
    /// price beyond minOut, and the attacker can only unwind at a loss. This
    /// mirrors:
    ///
    /// `cargo run --release -- simulate --victim 1000 --slippage 0.01 --attacker 2000`
    function test_oversized_frontrun_reverts_and_unwinds_at_loss() public {
        uint256 honestQuote = pool.quoteXForY(1_000 * UNIT);
        uint256 minOut = (honestQuote * 99) / 100;

        uint256 attackerXBefore = X.balanceOf(attacker);

        vm.prank(attacker);
        uint256 frontOut = pool.swapXForY(OVERSIZED_A_WAD, 0);
        assertGt(frontOut, 0, "front out > 0");
        console2.log("oversized front-run in X (wad)", OVERSIZED_A_WAD);
        console2.log("oversized front-run out Y (wad)", frontOut);

        uint256 victimPreview = pool.quoteXForY(1_000 * UNIT);
        assertLt(victimPreview, minOut, "oversized front-run crosses minOut");
        vm.expectRevert(bytes("slippage"));
        vm.prank(victim);
        pool.swapXForY(1_000 * UNIT, minOut);

        vm.prank(attacker);
        uint256 unwindOut = pool.swapYForX(frontOut, 0);
        console2.log("attacker unwind out X (wad)", unwindOut);

        uint256 attackerXAfter = X.balanceOf(attacker);
        assertLt(attackerXAfter, attackerXBefore, "attacker lost X after unwind");

        uint256 loss = attackerXBefore - attackerXAfter;
        console2.log("attacker unwind loss X (wad)", loss);
        _assertClose(loss, RUST_OVERSIZED_LOSS_WAD, RUST_OVERSIZED_LOSS_WAD / 100, "oversized loss within 1% of rust");

        assertEq(Y.balanceOf(attacker), 0, "attacker fully unwound Y position");
    }

    function _assertClose(uint256 got, uint256 want, uint256 tol, string memory tag) internal pure {
        uint256 diff = got > want ? got - want : want - got;
        require(diff <= tol, string.concat("mismatch: ", tag));
    }
}
