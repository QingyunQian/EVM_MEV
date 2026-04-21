// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

interface IERC20 {
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
    function transfer(address to, uint256 amount) external returns (bool);
    function balanceOf(address who) external view returns (uint256);
}

/// @notice Uniswap-V2 style constant-product AMM, fee on input side.
///         Pared down for teaching: no LP tokens, no flash swaps, no fee-on-transfer
///         tokens. `feeBps` is stored in basis points so 30 bps = 0.30%.
contract MiniAMM {
    IERC20 public immutable tokenX;
    IERC20 public immutable tokenY;
    uint256 public reserveX;
    uint256 public reserveY;
    uint16 public immutable feeBps;

    event Swap(address indexed who, bool xForY, uint256 amountIn, uint256 amountOut);
    event LiquidityAdded(address indexed who, uint256 dx, uint256 dy);

    constructor(address _tokenX, address _tokenY, uint16 _feeBps) {
        require(_feeBps < 10_000, "fee");
        tokenX = IERC20(_tokenX);
        tokenY = IERC20(_tokenY);
        feeBps = _feeBps;
    }

    function addLiquidity(uint256 dx, uint256 dy) external {
        require(tokenX.transferFrom(msg.sender, address(this), dx), "xIn");
        require(tokenY.transferFrom(msg.sender, address(this), dy), "yIn");
        reserveX += dx;
        reserveY += dy;
        emit LiquidityAdded(msg.sender, dx, dy);
    }

    /// @notice Swap exact `amountIn` of X for Y. Reverts if out < minOut.
    function swapXForY(uint256 amountIn, uint256 minOut) external returns (uint256 amountOut) {
        amountOut = _quoteXForY(amountIn);
        require(amountOut >= minOut, "slippage");
        require(tokenX.transferFrom(msg.sender, address(this), amountIn), "xIn");
        reserveX += amountIn;
        reserveY -= amountOut;
        require(tokenY.transfer(msg.sender, amountOut), "yOut");
        emit Swap(msg.sender, true, amountIn, amountOut);
    }

    function swapYForX(uint256 amountIn, uint256 minOut) external returns (uint256 amountOut) {
        amountOut = _quoteYForX(amountIn);
        require(amountOut >= minOut, "slippage");
        require(tokenY.transferFrom(msg.sender, address(this), amountIn), "yIn");
        reserveY += amountIn;
        reserveX -= amountOut;
        require(tokenX.transfer(msg.sender, amountOut), "xOut");
        emit Swap(msg.sender, false, amountIn, amountOut);
    }

    function quoteXForY(uint256 amountIn) external view returns (uint256) {
        return _quoteXForY(amountIn);
    }

    function quoteYForX(uint256 amountIn) external view returns (uint256) {
        return _quoteYForX(amountIn);
    }

    function _quoteXForY(uint256 amountIn) internal view returns (uint256) {
        uint256 amountInEff = amountIn * (10_000 - feeBps);
        uint256 numerator = amountInEff * reserveY;
        uint256 denominator = reserveX * 10_000 + amountInEff;
        return numerator / denominator;
    }

    function _quoteYForX(uint256 amountIn) internal view returns (uint256) {
        uint256 amountInEff = amountIn * (10_000 - feeBps);
        uint256 numerator = amountInEff * reserveX;
        uint256 denominator = reserveY * 10_000 + amountInEff;
        return numerator / denominator;
    }
}
