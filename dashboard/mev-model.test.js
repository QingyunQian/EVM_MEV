const assert = require("assert");
const MevModel = require("./mev-model.js");

const referenceInput = {
  victim: 1000,
  slippage: 0.01,
  depth: 100000,
  fee: 0.003,
  gasUnits: 0,
  baseFeeGwei: 0,
  priorityFeeGwei: 0,
  gasTokenPriceInX: 1
};

function approx(actual, expected, epsilon = 1e-6) {
  assert.ok(
    Math.abs(actual - expected) <= epsilon,
    `expected ${actual} to be within ${epsilon} of ${expected}`
  );
}

function testReferenceScenario() {
  const outcome = MevModel.optimizeGrossProfitWithGasCheck(referenceInput);

  approx(outcome.attackerIn, 507.044775, 1e-5);
  approx(outcome.victimActual, 977.286454, 1e-5);
  approx(outcome.profit, 7.016249, 1e-5);
  assert.strictEqual(outcome.reverted, false);
  assert.strictEqual(outcome.statusKind, "boundary");
  assert.strictEqual(outcome.statusLabel, "EXECUTES AT BOUNDARY");
  assert.strictEqual(
    outcome.clearanceLabel,
    "Executes at slippage boundary: victim output matches minOut within display tolerance."
  );
}

function testVictimLossComesFromWorseExecution() {
  const attacked = MevModel.simulate(referenceInput, 300);

  assert.ok(attacked.noAttackVictimOutput > attacked.victimActual);
  approx(attacked.victimLoss, attacked.noAttackVictimOutput - attacked.victimActual);
  approx(attacked.victimLossPct, attacked.victimLoss / attacked.noAttackVictimOutput);
}

function testRevertWhenBelowMinOut() {
  const outcome = MevModel.simulate(referenceInput, 2000);

  assert.strictEqual(outcome.reverted, true);
  assert.ok(outcome.victimActual < outcome.minOut);
  assert.ok(outcome.victimClearance < 0);
  assert.strictEqual(outcome.statusKind, "revert");
  assert.strictEqual(outcome.statusLabel, "REVERTS: MINOUT NOT SATISFIED");
  assert.strictEqual(outcome.clearanceLabel, "Reverts: victim output is below minOut");
}

function testNetProfitEqualsGrossMinusGas() {
  const input = {
    ...referenceInput,
    gasUnits: 500000,
    baseFeeGwei: 25,
    priorityFeeGwei: 2
  };
  const outcome = MevModel.simulate(input, 300);

  approx(outcome.gasCost, 0.0135);
  approx(outcome.netProfit, outcome.profit - outcome.gasCost);
}

function testStatusVariants() {
  const profitable = MevModel.simulate(referenceInput, 300);
  assert.strictEqual(profitable.statusLabel, "EXECUTES / PROFITABLE");

  const unprofitable = MevModel.simulate(
    { ...referenceInput, gasUnits: 500000, baseFeeGwei: 20000 },
    300
  );
  assert.strictEqual(unprofitable.statusLabel, "EXECUTES / NOT PROFITABLE");

  const skipped = MevModel.optimizeGrossProfitWithGasCheck({
    ...referenceInput,
    gasUnits: 500000,
    baseFeeGwei: 20000
  });
  assert.strictEqual(skipped.statusKind, "not_profitable");
  assert.strictEqual(skipped.statusLabel, "EXECUTES / NOT PROFITABLE");
  assert.ok(skipped.clearanceLabel.includes("not worth executing after gas"));
  assert.strictEqual(skipped.attackerIn, 0);

  const invalidSize = MevModel.simulate(referenceInput, referenceInput.depth * 10 + 1);
  assert.strictEqual(invalidSize.statusLabel, "INVALID: ATTACKER SIZE TOO LARGE");
}

function testFrontierInfeasibleRegionStartsAtMinOutFailure() {
  const frontier = MevModel.frontierRows(referenceInput, 2500, 200);
  const firstInfeasibleIndex = frontier.findIndex(row => row.infeasible);
  const region = MevModel.frontierInfeasibleRegion(referenceInput, 2500);

  assert.ok(firstInfeasibleIndex > 0, "expected an infeasible region");
  assert.ok(frontier[firstInfeasibleIndex].victimActual < frontier[firstInfeasibleIndex].minOut);
  assert.ok(frontier[firstInfeasibleIndex - 1].victimActual >= frontier[firstInfeasibleIndex - 1].minOut);
  assert.ok(region.shadeStart > region.maxFeasibleAttackerSize);
}

function testProfitSummaryUsesOutcomeValues() {
  const outcome = MevModel.optimizeGrossProfitWithGasCheck(referenceInput);
  const summary = MevModel.attackerProfitSummary(outcome, n => n.toFixed(3));

  assert.ok(summary.includes("514.061 X - 507.045 X = 7.016 X"));
  assert.ok(summary.includes("back-run received X - front-run spent X"));
}

function testStateRowsIncludeTradeDetails() {
  const outcome = MevModel.simulate(referenceInput, 300);
  const frontRun = outcome.states.find(row => row.action === "Front-run");
  const victim = outcome.states.find(row => row.action === "Victim swap");
  const backRun = outcome.states.find(row => row.action === "Back-run");

  assert.deepStrictEqual(
    [frontRun.actor, frontRun.direction, frontRun.input.token, frontRun.output.token],
    ["attacker", "X → Y", "X", "Y"]
  );
  assert.deepStrictEqual(
    [victim.actor, victim.direction, victim.input.token, victim.output.token],
    ["victim", "X → Y", "X", "Y"]
  );
  assert.deepStrictEqual(
    [backRun.actor, backRun.direction, backRun.input.token, backRun.output.token],
    ["attacker", "Y → X", "Y", "X"]
  );
}

function testStatePricesUseXPerY() {
  const outcome = MevModel.simulate(referenceInput, 300);
  const initial = outcome.states.find(row => row.action === "Initial pool");
  const frontRun = outcome.states.find(row => row.action === "Front-run");
  const victim = outcome.states.find(row => row.action === "Victim swap");
  const backRun = outcome.states.find(row => row.action === "Back-run");

  approx(initial.price, 1);
  assert.ok(frontRun.price > initial.price, "buying Y with X should raise the X/Y price");
  assert.ok(victim.price > frontRun.price, "victim buying Y should raise the X/Y price further");
  assert.ok(backRun.price < victim.price, "selling Y back to X should lower the X/Y price");
}

testReferenceScenario();
testVictimLossComesFromWorseExecution();
testRevertWhenBelowMinOut();
testNetProfitEqualsGrossMinusGas();
testStatusVariants();
testFrontierInfeasibleRegionStartsAtMinOutFailure();
testStateRowsIncludeTradeDetails();
testStatePricesUseXPerY();
testProfitSummaryUsesOutcomeValues();

console.log("mev-model tests passed");
