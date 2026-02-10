import { describe, expect, it } from 'vitest';
import {
  deriveCounterDialogContextFromTokenlog,
  encodeActionTokens,
  findActiveCounterChain,
  formatTokenlogForHistory,
  parseTokenlogActions,
} from '@/routes/cutthroat/helpers/tokenlog';

describe('cutthroat tokenlog helpers', () => {
  it('parses tokenlog actions and derives counter context chain', () => {
    const tokenlog = [
      'V1 CUTTHROAT3P DEALER P0 DECK AC AD AH AS ENDDECK',
      'P0 oneOff 4C P2',
      'P1 counter 2H',
      'P2 resolve',
    ].join(' ');

    const parsed = parseTokenlogActions(tokenlog);
    expect(parsed[0]).toEqual({
      type: 'ONEOFF',
      seat: 0,
      cardToken: '4C',
      target: {
        type: 'Player',
        seat: 2,
      },
    });
    expect(parsed[1]).toEqual({
      type: 'COUNTER_TWO',
      seat: 1,
      cardToken: '2H',
    });
    expect(parsed[2]).toEqual({
      type: 'COUNTER_PASS',
      seat: 2,
    });

    expect(findActiveCounterChain(parsed)).toEqual({
      oneOffCardToken: '4C',
      oneOffTarget: {
        type: 'Player',
        seat: 2,
      },
      twosPlayed: [ '2H' ],
    });
    expect(deriveCounterDialogContextFromTokenlog(tokenlog)).toEqual({
      oneOffCardToken: '4C',
      oneOffTarget: {
        type: 'Player',
        seat: 2,
      },
      twosPlayed: [ '2H' ],
    });
  });

  it('supports R7 one-off parsing and target extraction', () => {
    const tokenlog = [
      'V1 CUTTHROAT3P DEALER P0 DECK AC AD AH AS ENDDECK',
      'P1 oneOff AC 9C',
      'P2 resolve',
    ].join(' ');
    const parsed = parseTokenlogActions(tokenlog);

    expect(parsed[0]).toEqual({
      type: 'ONEOFF',
      seat: 1,
      cardToken: 'AC',
      target: {
        type: 'Point',
        token: '9C',
      },
    });
    expect(deriveCounterDialogContextFromTokenlog(tokenlog)).toEqual({
      oneOffCardToken: 'AC',
      oneOffTarget: {
        type: 'Point',
        token: '9C',
      },
      twosPlayed: [],
    });
  });

  it('returns tokenlog line for history and throws on malformed tokenlog', () => {
    const line = 'V1 CUTTHROAT3P DEALER P0 DECK AC ENDDECK P0 resolve';
    expect(formatTokenlogForHistory(line)).toEqual([ line ]);
    expect(formatTokenlogForHistory('')).toEqual([]);
    expect(() => parseTokenlogActions('V1 CUTTHROAT3P DEALER P0 DECK BAD ENDDECK')).toThrow('Invalid card token');
    expect(deriveCounterDialogContextFromTokenlog('V1 CUTTHROAT3P DEALER P0 DECK BAD ENDDECK')).toBeNull();
  });

  it('supports replay-scoped counter context by limiting actions', () => {
    const tokenlog = [
      'V1 CUTTHROAT3P DEALER P0 DECK AC AD AH AS ENDDECK',
      'P0 oneOff 4C P2',
      'P1 counter 2H',
      'P2 resolve',
    ].join(' ');

    expect(deriveCounterDialogContextFromTokenlog(tokenlog, 0)).toBeNull();
    expect(deriveCounterDialogContextFromTokenlog(tokenlog, 1)).toEqual({
      oneOffCardToken: '4C',
      oneOffTarget: {
        type: 'Player',
        seat: 2,
      },
      twosPlayed: [],
    });
    expect(deriveCounterDialogContextFromTokenlog(tokenlog, 2)).toEqual({
      oneOffCardToken: '4C',
      oneOffTarget: {
        type: 'Player',
        seat: 2,
      },
      twosPlayed: [ '2H' ],
    });
  });

  it('supports mixed resolve-four and resolve-five discard token shapes', () => {
    const tokenlog = [
      'V1 CUTTHROAT3P DEALER P0 DECK AC AD AH AS ENDDECK',
      'P1 resolve discard 7H',
      'P1 discard 6C',
    ].join(' ');
    const parsed = parseTokenlogActions(tokenlog);
    expect(parsed).toEqual([
      { type: 'OTHER', seat: 1, cardToken: '7H' },
      { type: 'OTHER', seat: 1, cardToken: '6C' },
    ]);

    expect(
      encodeActionTokens(
        { type: 'ResolveFourDiscard', data: { card: '7H' } },
        1,
        { type: 'ResolvingFour', data: { seat: 1, base_player: 0, remaining: 2 } },
      ),
    ).toBe('P1 resolve discard 7H');
    expect(
      encodeActionTokens(
        { type: 'ResolveFiveDiscard', data: { card: '6C' } },
        1,
        { type: 'ResolvingFive', data: { seat: 1, base_player: 1, discarded: false } },
      ),
    ).toBe('P1 discard 6C');
  });

  it('parses draw tokens with explicit and redacted cards', () => {
    const tokenlog = [
      'V1 CUTTHROAT3P DEALER P0 DECK AC AD AH AS ENDDECK',
      'P0 draw AC',
      'P1 draw UNKNOWN',
      'P2 pass',
    ].join(' ');
    const parsed = parseTokenlogActions(tokenlog);
    expect(parsed).toEqual([
      { type: 'OTHER', seat: 0, cardToken: 'AC' },
      { type: 'OTHER', seat: 1, cardToken: 'UNKNOWN' },
      { type: 'OTHER', seat: 2 },
    ]);
  });
});
