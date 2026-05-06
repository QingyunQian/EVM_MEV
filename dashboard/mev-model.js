(function initMevModel(root, factory) {
  const model = factory();
  if (typeof module === "object" && module.exports) {
    module.exports = model;
  }
  root.MevModel = model;
})(typeof globalThis !== "undefined" ? globalThis : window, function buildMevModel() {
  const DEFAULT_EPSILON = 1e-6;

  function priceOf(pool) {
    return pool.x / pool.y;
  }

  function previewXForY(pool, dx) {
    const dxEff = dx * (1 - pool.fee);
    return pool.y * dxEff / (pool.x + dxEff);
  }

  function previewYForX(pool, dy) {
    const dyEff = dy * (1 - pool.fee);
    return pool.x * dyEff / (pool.y + dyEff);
  }

  function swapXForY(pool, dx) {
    const out = previewXForY(pool, dx);
    pool.x += dx;
    pool.y -= out;
    return out;
  }

  function swapYForX(pool, dy) {
    const out = previewYForX(pool, dy);
    pool.y += dy;
    pool.x -= out;
    return out;
  }

  function gasCostInX(input) {
    return input.gasUnits * (input.baseFeeGwei + input.priorityFeeGwei) * 1e-9 * input.gasTokenPriceInX;
  }

  function amount(value, token) {
    return { value, token };
  }

  function stateRow(step, action, actor, direction, inputAmount, outputAmount, pool) {
    return {
      step,
      action,
      actor,
      direction,
      input: inputAmount,
      output: outputAmount,
      x: pool.x,
      y: pool.y,
      price: priceOf(pool)
    };
  }

  function enrichOutcome(input, outcome, skippedByGas = false, grossCandidate = null) {
    outcome.noAttackVictimOutput = outcome.honestOut;
    outcome.victimLoss = outcome.noAttackVictimOutput - outcome.victimActual;
    outcome.victimLossPct = outcome.noAttackVictimOutput > 0
      ? outcome.victimLoss / outcome.noAttackVictimOutput
      : 0;
    outcome.victimClearance = outcome.victimActual - outcome.minOut;
    outcome.maxAttackerCap = input.depth * 10;
    outcome.skippedByGas = skippedByGas;
    outcome.grossCandidate = grossCandidate || outcome;
    Object.assign(outcome, computeStatus(outcome, DEFAULT_EPSILON));
    return outcome;
  }

  function simulate(input, attackerIn) {
    const start = { x: input.depth, y: input.depth, fee: input.fee };
    const honestOut = previewXForY(start, input.victim);
    const minOut = honestOut * (1 - input.slippage);
    const states = [
      stateRow("0", "Initial pool", "-", "-", amount(0, "-"), amount(0, "-"), start)
    ];

    if (attackerIn <= 0) {
      const p0 = { ...start };
      const victimActual = swapXForY(p0, input.victim);
      states.push(stateRow("1", "Victim swap", "victim", "X → Y", amount(input.victim, "X"), amount(victimActual, "Y"), p0));
      return enrichOutcome(input, {
        attackerIn: 0,
        frontOut: 0,
        backOut: 0,
        profit: 0,
        gasCost: 0,
        netProfit: 0,
        roi: 0,
        honestOut,
        minOut,
        victimActual,
        reverted: false,
        states
      });
    }

    const p = { ...start };
    const frontOut = swapXForY(p, attackerIn);
    states.push(stateRow("1", "Front-run", "attacker", "X → Y", amount(attackerIn, "X"), amount(frontOut, "Y"), p));
    const gasCost = gasCostInX(input);

    const victimPreview = previewXForY(p, input.victim);
    if (victimPreview < minOut) {
      states.push(stateRow("2", "Victim reverts", "victim", "X → Y", amount(input.victim, "X"), amount(victimPreview, "Y"), p));
      const backOut = swapYForX(p, frontOut);
      const profit = backOut - attackerIn;
      states.push(stateRow("3", "Unwind", "attacker", "Y → X", amount(frontOut, "Y"), amount(backOut, "X"), p));
      return enrichOutcome(input, {
        attackerIn,
        frontOut,
        backOut,
        profit,
        gasCost,
        netProfit: profit - gasCost,
        roi: profit / attackerIn,
        honestOut,
        minOut,
        victimActual: victimPreview,
        reverted: true,
        states
      });
    }

    const victimActual = swapXForY(p, input.victim);
    states.push(stateRow("2", "Victim swap", "victim", "X → Y", amount(input.victim, "X"), amount(victimActual, "Y"), p));
    const backOut = swapYForX(p, frontOut);
    const profit = backOut - attackerIn;
    states.push(stateRow("3", "Back-run", "attacker", "Y → X", amount(frontOut, "Y"), amount(backOut, "X"), p));
    return enrichOutcome(input, {
      attackerIn,
      frontOut,
      backOut,
      profit,
      gasCost,
      netProfit: profit - gasCost,
      roi: profit / attackerIn,
      honestOut,
      minOut,
      victimActual,
      reverted: false,
      states
    });
  }

  function computeStatus(outcome, epsilon = DEFAULT_EPSILON) {
    if (outcome.skippedByGas) {
      return {
        statusKind: "not_profitable",
        statusLabel: "EXECUTES / NOT PROFITABLE",
        clearanceLabel: "Executes, but the sandwich is not worth executing after gas"
      };
    }

    if (outcome.attackerIn > outcome.maxAttackerCap) {
      return {
        statusKind: "invalid_attacker_size",
        statusLabel: "INVALID: ATTACKER SIZE TOO LARGE",
        clearanceLabel: "Invalid: attacker size exceeds the model's search cap"
      };
    }

    if (outcome.reverted || outcome.victimClearance < -epsilon) {
      return {
        statusKind: "revert",
        statusLabel: "REVERTS: MINOUT NOT SATISFIED",
        clearanceLabel: "Reverts: victim output is below minOut"
      };
    }

    if (Math.abs(outcome.victimClearance) <= epsilon && outcome.attackerIn > 0) {
      return {
        statusKind: "boundary",
        statusLabel: "EXECUTES AT BOUNDARY",
        clearanceLabel: "Executes at slippage boundary: victim output matches minOut within display tolerance."
      };
    }

    if (outcome.attackerIn > 0 && outcome.netProfit > 0) {
      return {
        statusKind: "profitable",
        statusLabel: "EXECUTES / PROFITABLE",
        clearanceLabel: `Executes: victim output clears minOut by +${outcome.victimClearance} Y`
      };
    }

    if (outcome.attackerIn > 0) {
      return {
        statusKind: "not_profitable",
        statusLabel: "EXECUTES / NOT PROFITABLE",
        clearanceLabel: `Executes: victim output clears minOut by +${Math.max(0, outcome.victimClearance)} Y`
      };
    }

    return {
      statusKind: "no_attack",
      statusLabel: "EXECUTES / NOT PROFITABLE",
      clearanceLabel: `Executes: victim output clears minOut by +${Math.max(0, outcome.victimClearance)} Y`
    };
  }

  function maxFeasible(input, hi) {
    if (!simulate(input, hi).reverted) return hi;
    let lo = 0;
    for (let i = 0; i < 64; i += 1) {
      const mid = (lo + hi) / 2;
      if (simulate(input, mid).reverted) hi = mid;
      else lo = mid;
    }
    return lo;
  }

  function optimizeGrossProfit(input) {
    const cap = input.depth * 10;
    const hi0 = maxFeasible(input, cap);
    if (hi0 <= 0) return simulate(input, 0);

    const phi = (Math.sqrt(5) - 1) / 2;
    let lo = 0;
    let hi = hi0;
    let c = hi - phi * (hi - lo);
    let d = lo + phi * (hi - lo);
    const profit = (a) => {
      const o = simulate(input, a);
      return o.reverted ? Number.NEGATIVE_INFINITY : o.profit;
    };

    for (let i = 0; i < 100; i += 1) {
      if (Math.abs(hi - lo) < 1e-10) break;
      if (profit(c) > profit(d)) hi = d;
      else lo = c;
      c = hi - phi * (hi - lo);
      d = lo + phi * (hi - lo);
    }

    return simulate(input, (lo + hi) / 2);
  }

  function optimizeGrossProfitWithGasCheck(input) {
    const candidate = optimizeGrossProfit(input);
    if (candidate.profit > candidate.gasCost) {
      candidate.grossCandidate = candidate;
      candidate.skippedByGas = false;
      Object.assign(candidate, computeStatus(candidate, DEFAULT_EPSILON));
      return candidate;
    }

    const skipped = simulate(input, 0);
    skipped.grossCandidate = candidate;
    skipped.skippedByGas = candidate.profit > 0;
    Object.assign(skipped, computeStatus(skipped, DEFAULT_EPSILON));
    return skipped;
  }

  function frontierRows(input, upper, steps = 160) {
    const rows = [];
    for (let i = 0; i <= steps; i += 1) {
      const a = upper * i / steps;
      const outcome = simulate(input, a);
      rows.push({
        a,
        ...outcome,
        infeasible: outcome.victimActual < outcome.minOut
      });
    }
    return rows;
  }

  function frontierInfeasibleRegion(input, upper) {
    const normalized = normalizeInput(input);
    const maxFeasibleAttackerSize = maxFeasible(normalized, normalized.depth * 10);
    const visualOffset = Math.max(upper * 0.004, DEFAULT_EPSILON);
    return {
      maxFeasibleAttackerSize,
      shadeStart: Math.min(upper, maxFeasibleAttackerSize + visualOffset)
    };
  }

  function attackerProfitSummary(outcome, format = n => Number(n).toFixed(3)) {
    if (!outcome || outcome.attackerIn <= 0) {
      return "No attacker trade is selected, so attacker gross profit is 0 X.";
    }
    const received = outcome.backOut;
    const spent = outcome.attackerIn;
    const label = outcome.reverted ? "Attacker gross PnL after unwind" : "Attacker gross profit";
    return `${label} = back-run received X - front-run spent X = ${format(received)} X - ${format(spent)} X = ${format(outcome.profit)} X`;
  }

  function normalizeInput(input) {
    return {
      victim: Number(input.victim),
      slippage: Number(input.slippage),
      depth: Number(input.depth),
      fee: Number(input.fee),
      gasUnits: Number(input.gasUnits || 0),
      baseFeeGwei: Number(input.baseFeeGwei || 0),
      priorityFeeGwei: Number(input.priorityFeeGwei || 0),
      gasTokenPriceInX: Number(input.gasTokenPriceInX ?? input.nativePriceX ?? 1)
    };
  }

  return {
    DEFAULT_EPSILON,
    priceOf,
    previewXForY,
    previewYForX,
    swapXForY,
    swapYForX,
    gasCostInX,
    simulate: (input, attackerIn) => simulate(normalizeInput(input), attackerIn),
    computeStatus,
    maxFeasible: (input, hi) => maxFeasible(normalizeInput(input), hi),
    optimizeGrossProfit: input => optimizeGrossProfit(normalizeInput(input)),
    optimizeGrossProfitWithGasCheck: input => optimizeGrossProfitWithGasCheck(normalizeInput(input)),
    frontierRows: (input, upper, steps) => frontierRows(normalizeInput(input), upper, steps),
    frontierInfeasibleRegion,
    attackerProfitSummary,
    normalizeInput
  };
});
