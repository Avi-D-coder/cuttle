import { describe, expect, it } from 'vitest';
import {
  deriveCounterDialogContextFromTokenlog,
  findActiveCounterChain,
  formatTokenlogForHistory,
  parseTokenlogActions,
} from '@/routes/cutthroat/helpers/tokenlog';

describe('cutthroat tokenlog helpers', () => {
  it('parses tokenlog actions and derives counter context chain', () => {
    const tokenlog = [
      'V1 CUTTHROAT3P DEALER P0 DECK AC AD AH AS ENDDECK',
      'P0 MT_ONEOFF 4C TGT_P P2',
      'P1 MT_C2 2H',
      'P2 MT_CPASS',
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
      'P1 MT_R7 SRC 0 AS ONEOFF TGT_POINT 9C',
      'P2 MT_CPASS',
    ].join(' ');
    const parsed = parseTokenlogActions(tokenlog);

    expect(parsed[0]).toEqual({
      type: 'ONEOFF',
      seat: 1,
      cardToken: null,
      sourceIndex: 0,
      target: {
        type: 'Point',
        token: '9C',
      },
    });
    expect(deriveCounterDialogContextFromTokenlog(tokenlog)).toBeNull();
  });

  it('returns tokenlog line for history and throws on malformed tokenlog', () => {
    const line = 'V1 CUTTHROAT3P DEALER P0 DECK AC ENDDECK P0 MT_CPASS';
    expect(formatTokenlogForHistory(line)).toEqual([ line ]);
    expect(formatTokenlogForHistory('')).toEqual([]);
    expect(() => parseTokenlogActions('V1 CUTTHROAT3P DEALER P0 DECK BAD ENDDECK')).toThrow('Invalid card token');
    expect(deriveCounterDialogContextFromTokenlog('V1 CUTTHROAT3P DEALER P0 DECK BAD ENDDECK')).toBeNull();
  });
});
