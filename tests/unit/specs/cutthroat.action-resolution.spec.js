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

describe('cutthroat action resolution helpers (token actions)', () => {
  it('extracts normalized action sources from tokenized legal actions', () => {
    expect(extractActionSource('P1 draw')).toEqual({ zone: 'deck' });
    expect(extractActionSource('P1 points 7C')).toEqual({ zone: 'hand', token: '7C' });
    expect(extractActionSource('P1 resolve')).toEqual({ zone: 'counter', token: 'pass' });
    expect(extractActionSource('P1 resolve 4D')).toEqual({ zone: 'scrap', token: '4D' });
    expect(extractActionSource('P1 points KD', 'ResolvingSeven')).toEqual({ zone: 'reveal', token: 'KD' });
    expect(extractActionSource('P1 stalemate-propose')).toEqual({ zone: 'stalemate', token: 'request' });
    expect(extractActionSource('P1 stalemate-accept')).toEqual({ zone: 'stalemate', token: 'accept' });
    expect(extractActionSource('P1 stalemate-reject')).toEqual({ zone: 'stalemate', token: 'reject' });
  });

  it('derives stable move choices for a selected source', () => {
    const actions = [
      'P1 scuttle 7C 5C',
      'P1 points 7C',
      'P1 oneOff 7C',
      'P1 playRoyal KC',
      'P1 resolve',
      'P1 stalemate-propose',
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
    expect(deriveMoveChoicesForSource(actions, { zone: 'stalemate', token: 'request' })).toEqual([
      { type: 'stalemateRequest' },
    ]);
  });

  it('derives targets and finds matching token action', () => {
    const actions = [
      'P1 oneOff 9C 7D',
      'P1 oneOff 9C QH',
      'P1 oneOff 9C JH',
      'P1 oneOff 9C J0',
      'P1 oneOff 9C P2',
      'P1 oneOff 9C',
      'P1 resolve discard 4C',
    ];

    expect(deriveTargetsForChoice(actions, { zone: 'hand', token: '9C' }, 'oneOff')).toEqual([
      { targetType: 'card', token: '7D', key: 'card:7D' },
      { targetType: 'card', token: 'QH', key: 'card:QH' },
      { targetType: 'card', token: 'JH', key: 'card:JH' },
      { targetType: 'card', token: 'J0', key: 'card:J0' },
      { targetType: 'player', seat: 2, key: 'player:2' },
    ]);
    expect(findMatchingAction(actions, { zone: 'hand', token: '9C' }, 'oneOff', { targetType: 'card', token: '7D' })).toBe('P1 oneOff 9C 7D');
    expect(findMatchingAction(actions, { zone: 'hand', token: '9C' }, 'oneOff', { targetType: 'point', token: '7D' })).toBe('P1 oneOff 9C 7D');
    expect(findMatchingAction(actions, { zone: 'hand', token: '9C' }, 'oneOff', { targetType: 'royal', token: 'QH' })).toBe('P1 oneOff 9C QH');
    expect(findMatchingAction(actions, { zone: 'hand', token: '9C' }, 'oneOff', { targetType: 'jack', token: 'JH' })).toBe('P1 oneOff 9C JH');
    expect(findMatchingAction(actions, { zone: 'hand', token: '9C' }, 'oneOff', { targetType: 'joker', token: 'J0' })).toBe('P1 oneOff 9C J0');
    expect(findMatchingAction(actions, { zone: 'hand', token: '9C' }, 'oneOff')).toBe('P1 oneOff 9C');
    expect(findMatchingAction(actions, { zone: 'hand', token: '4C' }, 'resolveFourDiscard')).toBe('P1 resolve discard 4C');
  });

  it('classifies joker targets as jack or royal from tokenized actions', () => {
    const actions = [
      'P1 playRoyal J0 JH',
      'P1 playRoyal J0 QH',
    ];
    const source = { zone: 'hand', token: 'J0' };

    expect(deriveTargetsForChoice(actions, source, 'joker')).toEqual([
      { targetType: 'jack', token: 'JH', key: 'jack:JH' },
      { targetType: 'royal', token: 'QH', key: 'royal:QH' },
    ]);
    expect(findMatchingAction(actions, source, 'joker', { targetType: 'jack', token: 'JH' })).toBe('P1 playRoyal J0 JH');
    expect(findMatchingAction(actions, source, 'joker', { targetType: 'royal', token: 'QH' })).toBe('P1 playRoyal J0 QH');
  });

  it('filters seven actions by selected reveal token and groups by choice category', () => {
    const actions = [ 'P1 points 3C', 'P1 resolve', 'P1 points KD', 'P1 discard 9C' ];
    const filtered = filterVisibleActions(actions, true, 'KD', 'ResolvingSeven');
    expect(filtered).toEqual([ 'P1 points KD' ]);
    const grouped = groupActions(actions);
    expect(grouped.primary).toEqual([ 'P1 points 3C', 'P1 points KD' ]);
    expect(grouped.counter).toEqual([ 'P1 resolve' ]);
    expect(grouped.resolution).toEqual([ 'P1 discard 9C' ]);
  });

  it('derives dialog state from tokenized legal actions only', () => {
    const withCounterTwo = deriveCutthroatDialogState({
      phaseType: 'Countering',
      legalActions: [ 'P1 resolve', 'P1 counter 2C' ],
    });
    expect(withCounterTwo.hasCounterPass).toBe(true);
    expect(withCounterTwo.counterTwoTokens).toEqual([ '2C' ]);
    expect(withCounterTwo.showCounterDialog).toBe(true);

    const resolvingFour = deriveCutthroatDialogState({
      phaseType: 'ResolvingFour',
      legalActions: [ 'P1 resolve discard 7C', 'P1 resolve discard 8D' ],
    });
    expect(resolvingFour.resolveFourTokens).toEqual([ '7C', '8D' ]);

    const resolvingFive = deriveCutthroatDialogState({
      phaseType: 'ResolvingFive',
      legalActions: [ 'P1 discard 9H' ],
    });
    expect(resolvingFive.resolveFiveTokens).toEqual([ '9H' ]);
  });

  it('keeps fallback logic for selected cards', () => {
    expect(deriveFallbackChoiceTypesForSelectedCard(
      { zone: 'hand', token: 'J0' },
      { kind: 'joker', id: 0 },
    )).toEqual([ 'joker' ]);
    expect(deriveFallbackChoiceTypesForSelectedCard(
      { zone: 'hand', token: '4C' },
      { kind: 'standard', rank: 4, suit: 0 },
    )).toEqual([ 'points', 'scuttle', 'oneOff' ]);
  });
});
