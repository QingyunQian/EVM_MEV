const assert = require("assert");
const fs = require("fs");

const html = fs.readFileSync("dashboard/index.html", "utf8");

function expectIncludes(text, message = `missing ${text}`) {
  assert.ok(html.includes(text), message);
}

function expectMatches(regex, message) {
  assert.ok(regex.test(html), message || `missing ${regex}`);
}

function testMetricCardsAvoidOverflow() {
  expectMatches(/grid-template-columns:\s*repeat\(auto-fit,\s*minmax\(180px,\s*1fr\)\)/, "metrics should use responsive min card width");
  expectMatches(/\.metric strong[\s\S]*white-space:\s*nowrap/, "metric values should keep token symbols with numbers");
  expectMatches(/font-size:\s*clamp\(16px,\s*1\.8vw,\s*24px\)/, "metric values should use readable responsive font sizing");
}

function testVisibleDisclaimer() {
  expectIncludes("Educational constant-product AMM simulator.", "visible simulator disclaimer should be in the header");
  expectIncludes("Does not model live mempool access", "disclaimer should clarify production limitations");
}

function testGasFormulaWraps() {
  expectMatches(/\.formula[\s\S]*white-space:\s*pre-wrap/, "gas formula should wrap instead of horizontal scrolling");
  expectMatches(/\.formula[\s\S]*word-break:\s*break-word/, "gas formula should break long words if needed");
  expectIncludes(`gasCostInX =
  gasUnits
  × (baseFeeGwei + priorityFeeGwei)
  × 1e-9
  × gasTokenPriceInX`);
}

function testSyntheticCandleLayout() {
  expectIncludes('<div id="eventTimeline" class="event-timeline"');
  expectIncludes("Each candle represents one simulated event");
  expectIncludes("Synthetic Event Candles");
  assert.ok(!html.includes("MEV BUY"));
  assert.ok(!html.includes("MEV SELL"));
  assert.ok(
    !/ctx\.fillText\(label,\s*labelX,\s*h - 25\)/.test(html),
    "event names should not be drawn as a crowded bottom axis row"
  );
  expectIncludes("drawBandLabel(ctx, cx, pad.t + 16, stage.marker)");
}

function testMevEventCandlesDoNotUseWicks() {
  expectMatches(
    /const wickScale = marker \? 0 : scale;/,
    "MEV event candles should not add synthetic upper/lower wicks"
  );
  expectIncludes("high: Math.max(open, close) + wickScale * upperBias * waveA");
  expectIncludes("low: Math.min(open, close) - wickScale * lowerBias * waveB");
  expectMatches(/appendAmbientCandles[\s\S]*candleWithWicks\([\s\S]*\n\s*null,\n\s*1\.7,/, "ambient candles should keep passing a null marker so their wicks remain");
}

function testDisplayTextPolish() {
  expectIncludes("X → Y");
  expectIncludes("Y → X");
  assert.ok(!html.includes('join(" -> ")'), "displayed price paths should use the same Unicode arrow style");
  assert.ok(!html.includes("`${clearanceText(selected)}."), "scenario banner should avoid adding a second period after boundary text");
  expectIncludes("appendSentence(clearanceText(selected)");
}

function testPriceLabelsUseXPerY() {
  expectIncludes("Price (X/Y)");
  expectIncludes("Price is shown as X per Y");
  expectIncludes("Price path (X/Y)");
  expectIncludes("Price (X/Y) Event Candles");
  assert.ok(!html.includes("Price (Y/X)"), "dashboard should not mix Y/X and X/Y labels");
  assert.ok(!html.includes("Y per X"), "price explanation should use X per Y");
  assert.ok(!html.includes("Y/X price"), "price movement copy should use X/Y");
}

function testFrontierBoundaryLanguage() {
  expectIncludes("Beyond this point: victim reverts");
  expectIncludes("Selected attacker size is at the slippage boundary.");
  expectMatches(/frontierInfeasibleRegion\(input,\s*upper\)/, "frontier should derive shading from max feasible boundary");
}

function testStateTableProfitSummary() {
  expectIncludes('id="profitSummary"');
  expectIncludes("attackerProfitSummary");
}

function testHighGasPresetIsActuallyHighGas() {
  expectMatches(
    /highGas:\s*\{[\s\S]*baseFee:\s*20000[\s\S]*priorityFee:\s*2/,
    "High gas preset should make the sandwich not profitable after gas without calling it invalid"
  );
}

testMetricCardsAvoidOverflow();
testVisibleDisclaimer();
testGasFormulaWraps();
testSyntheticCandleLayout();
testMevEventCandlesDoNotUseWicks();
testDisplayTextPolish();
testPriceLabelsUseXPerY();
testFrontierBoundaryLanguage();
testStateTableProfitSummary();
testHighGasPresetIsActuallyHighGas();

console.log("dashboard UI tests passed");
