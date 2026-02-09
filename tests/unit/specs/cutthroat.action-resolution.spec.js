import { describe, expect, it } from 'vitest';
import {
  deriveFallbackChoiceTypesForSelectedCard,
  deriveMoveChoicesForSource,
  deriveCutthroatDialogState,
  deriveTargetsForChoice,
  extractActionSource,
  filterVisibleActions,
  findMatchingAction,
  groupActions,
} from '@/routes/cutthroat/helpers/action-resolution';

describe('cutthroat action resolution helpers', () => {
  it('extracts normalized action sources for deck, hand, counter, scrap, and reveal', () => {
    expect(extractActionSource({ type: 'Draw' })).toEqual({ zone: 'deck' });
    expect(extractActionSource({ type: 'PlayPoints', data: { card: '7C' } })).toEqual({
      zone: 'hand',
      token: '7C',
    });
    expect(extractActionSource({ type: 'CounterPass' })).toEqual({
      zone: 'counter',
      token: 'pass',
    });
    expect(extractActionSource({ type: 'ResolveThreePick', data: { card_from_scrap: '4D' } })).toEqual({
      zone: 'scrap',
      token: '4D',
    });
    expect(extractActionSource({ type: 'ResolveSevenChoose', data: { source_index: 1, play: { type: 'Points' } } })).toEqual({
      zone: 'reveal',
      index: 1,
    });
  });

  it('derives stable move choices for a selected source', () => {
    const actions = [
      { type: 'Scuttle', data: { card: '7C', target_point_base: '5C' } },
      { type: 'PlayPoints', data: { card: '7C' } },
      { type: 'PlayOneOff', data: { card: '7C', target: { type: 'None' } } },
      { type: 'PlayRoyal', data: { card: 'KC' } },
      { type: 'CounterPass' },
    ];

    expect(deriveMoveChoicesForSource(actions, { zone: 'hand', token: '7C' })).toEqual([
      { type: 'points' },
      { type: 'scuttle' },
      { type: 'oneOff' },
    ]);

    expect(deriveMoveChoicesForSource(actions, { zone: 'counter', token: 'pass' })).toEqual([
      { type: 'counterPass' },
    ]);

    expect(deriveMoveChoicesForSource(actions, { zone: 'hand', token: 'KC' })).toEqual([
      { type: 'royal' },
    ]);
  });

  it('derives joker fallback move choice for selected joker hand card', () => {
    expect(deriveFallbackChoiceTypesForSelectedCard(
      { zone: 'hand', token: 'J0' },
      { kind: 'joker', id: 0 },
    )).toEqual([ 'joker' ]);
  });

  it('derives existing standard-rank fallback move choices for selected hand card', () => {
    expect(deriveFallbackChoiceTypesForSelectedCard(
      { zone: 'hand', token: '4C' },
      { kind: 'standard', rank: 4, suit: 0 },
    )).toEqual([ 'points', 'scuttle', 'oneOff' ]);
  });

  it('returns no fallback choices for non-hand sources', () => {
    expect(deriveFallbackChoiceTypesForSelectedCard(
      { zone: 'reveal', index: 0 },
      { kind: 'joker', id: 0 },
    )).toEqual([]);
  });

  it('derives unique targets for targeted choices', () => {
    const actions = [
      {
        type: 'PlayOneOff',
        data: {
          card: '9C',
          target: { type: 'Point', data: { base: '7D' } },
        },
      },
      {
        type: 'PlayOneOff',
        data: {
          card: '9C',
          target: { type: 'Player', data: { seat: 2 } },
        },
      },
      {
        type: 'PlayOneOff',
        data: {
          card: '9C',
          target: { type: 'None' },
        },
      },
      {
        type: 'ResolveSevenChoose',
        data: {
          source_index: 0,
          play: { type: 'Scuttle', data: { target: '6S' } },
        },
      },
    ];

    expect(deriveTargetsForChoice(actions, { zone: 'hand', token: '9C' }, 'oneOff')).toEqual([
      {
        targetType: 'point',
        token: '7D',
        key: 'point:7D',
      },
      {
        targetType: 'player',
        seat: 2,
        key: 'player:2',
      },
    ]);

    expect(deriveTargetsForChoice(actions, { zone: 'reveal', index: 0 }, 'scuttle')).toEqual([
      {
        targetType: 'point',
        token: '6S',
        key: 'point:6S',
      },
    ]);
  });

  it('finds matching actions for targeted and targetless choices', () => {
    const actions = [
      {
        type: 'PlayOneOff',
        data: {
          card: '9C',
          target: { type: 'Point', data: { base: '7D' } },
        },
      },
      {
        type: 'PlayOneOff',
        data: {
          card: '9C',
          target: { type: 'None' },
        },
      },
      {
        type: 'ResolveFourDiscard',
        data: { card: '4C' },
      },
    ];

    expect(findMatchingAction(actions, { zone: 'hand', token: '9C' }, 'oneOff', {
      targetType: 'point',
      token: '7D',
    })).toEqual(actions[0]);

    expect(findMatchingAction(actions, { zone: 'hand', token: '9C' }, 'oneOff')).toEqual(actions[1]);

    expect(findMatchingAction(actions, { zone: 'hand', token: '4C' }, 'resolveFourDiscard')).toEqual(actions[2]);
  });

  it('filters seven actions and groups action sections', () => {
    const actions = [
      { type: 'PlayPoints', data: { card: '3C' } },
      { type: 'CounterPass' },
      { type: 'ResolveSevenChoose', data: { source_index: 0, play: { type: 'Points' } } },
      { type: 'ResolveSevenChoose', data: { source_index: 1, play: { type: 'Discard' } } },
    ];

    const filtered = filterVisibleActions(actions, true, 0);
    expect(filtered).toEqual([
      { type: 'ResolveSevenChoose', data: { source_index: 0, play: { type: 'Points' } } },
    ]);

    const grouped = groupActions(actions);
    expect(grouped.primary).toEqual([ { type: 'PlayPoints', data: { card: '3C' } } ]);
    expect(grouped.counter).toEqual([ { type: 'CounterPass' } ]);
    expect(grouped.resolution).toEqual([
      { type: 'ResolveSevenChoose', data: { source_index: 0, play: { type: 'Points' } } },
      { type: 'ResolveSevenChoose', data: { source_index: 1, play: { type: 'Discard' } } },
    ]);
    expect(grouped.other).toEqual([]);
  });

  it('derives action-driven counter dialog state', () => {
    const withCounterTwo = deriveCutthroatDialogState({
      phaseType: 'Countering',
      legalActions: [
        { type: 'CounterPass' },
        { type: 'CounterTwo', data: { two_card: '2C' } },
      ],
    });
    expect(withCounterTwo.hasCounterPass).toBe(true);
    expect(withCounterTwo.counterTwoTokens).toEqual([ '2C' ]);
    expect(withCounterTwo.showCounterDialog).toBe(true);
    expect(withCounterTwo.showCannotCounterDialog).toBe(false);

    const withoutCounterTwo = deriveCutthroatDialogState({
      phaseType: 'Countering',
      legalActions: [ { type: 'CounterPass' } ],
    });
    expect(withoutCounterTwo.hasCounterPass).toBe(true);
    expect(withoutCounterTwo.counterTwoTokens).toEqual([]);
    expect(withoutCounterTwo.showCounterDialog).toBe(false);
    expect(withoutCounterTwo.showCannotCounterDialog).toBe(true);
  });

  it('derives resolve dialog state from legal actions only', () => {
    const state = deriveCutthroatDialogState({
      phaseType: 'ResolvingFour',
      legalActions: [
        { type: 'ResolveFourDiscard', data: { card: '7C' } },
        { type: 'ResolveFourDiscard', data: { card: '8D' } },
      ],
    });
    expect(state.resolveFourTokens).toEqual([ '7C', '8D' ]);
    expect(state.showResolveFourDialog).toBe(true);
    expect(state.showResolveFiveDialog).toBe(false);

    const resolvingFive = deriveCutthroatDialogState({
      phaseType: 'ResolvingFive',
      legalActions: [ { type: 'ResolveFiveDiscard', data: { card: '9H' } } ],
    });
    expect(resolvingFive.resolveFiveTokens).toEqual([ '9H' ]);
    expect(resolvingFive.showResolveFiveDialog).toBe(true);
  });

  it('derives rank-4 player target dialog without metadata dependencies', () => {
    const show = deriveCutthroatDialogState({
      phaseType: 'Main',
      legalActions: [],
      selectedSource: { zone: 'hand', token: '4C' },
      selectedChoice: 'oneOff',
      targets: [
        { targetType: 'player', seat: 1, key: 'player:1' },
        { targetType: 'player', seat: 2, key: 'player:2' },
      ],
    });
    expect(show.playerTargetSeats).toEqual([ 1, 2 ]);
    expect(show.showFourPlayerTargetDialog).toBe(true);

    const hide = deriveCutthroatDialogState({
      phaseType: 'Main',
      legalActions: [],
      selectedSource: { zone: 'hand', token: '5C' },
      selectedChoice: 'oneOff',
      targets: [ { targetType: 'player', seat: 1, key: 'player:1' } ],
    });
    expect(hide.showFourPlayerTargetDialog).toBe(false);
  });
});
