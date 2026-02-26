import { describe, expect, it } from 'vitest';
import {
  deriveCounterDialogContextFromPhase,
  deriveCounterDialogContextFromTokenlog,
  deriveLatestOneOffContextFromTokenlog,
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
      oneOffSeat: 0,
      triggeringSeat: 1,
      oneOffCardToken: '4C',
      oneOffTarget: {
        type: 'Player',
        seat: 2,
      },
      twosPlayed: [ '2H' ],
    });
    expect(deriveCounterDialogContextFromTokenlog(tokenlog)).toEqual({
      oneOffSeat: 0,
      triggeringSeat: 1,
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
      oneOffSeat: 1,
      triggeringSeat: 1,
      oneOffCardToken: 'AC',
      oneOffTarget: {
        type: 'Point',
        token: '9C',
      },
      twosPlayed: [],
    });
  });

  it('does not consume following seat token for untargeted non-four one-offs', () => {
    const tokenlog = [
      'V1 CUTTHROAT3P DEALER P0 DECK AC AD AH AS ENDDECK',
      'P0 oneOff 7D',
      'P1 points TS',
    ].join(' ');

    expect(parseTokenlogActions(tokenlog)).toEqual([
      {
        type: 'ONEOFF',
        seat: 0,
        cardToken: '7D',
        target: {
          type: 'None',
        },
      },
      {
        type: 'OTHER',
        seat: 1,
        cardToken: 'TS',
      },
    ]);

    expect(deriveLatestOneOffContextFromTokenlog(tokenlog)).toEqual({
      oneOffSeat: 0,
      oneOffCardToken: '7D',
      oneOffTarget: {
        type: 'None',
      },
    });
  });

  it('keeps player target parsing for four one-offs', () => {
    const tokenlog = [
      'V1 CUTTHROAT3P DEALER P0 DECK AC AD AH AS ENDDECK',
      'P0 oneOff 4C P2',
      'P1 resolve',
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
  });

  it('formats verbose history lines and throws on malformed tokenlog', () => {
    const line = 'V1 CUTTHROAT3P DEALER P0 DECK AC ENDDECK P1 draw';
    expect(formatTokenlogForHistory(line, { seatNames: { 0: 'Dealer', 1: 'Starter' } })).toEqual([
      'Dealer dealt; Starter will go first',
      'Starter drew a card.',
    ]);
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
      oneOffSeat: 0,
      triggeringSeat: 0,
      oneOffCardToken: '4C',
      oneOffTarget: {
        type: 'Player',
        seat: 2,
      },
      twosPlayed: [],
    });
    expect(deriveCounterDialogContextFromTokenlog(tokenlog, 2)).toEqual({
      oneOffSeat: 0,
      triggeringSeat: 1,
      oneOffCardToken: '4C',
      oneOffTarget: {
        type: 'Player',
        seat: 2,
      },
      twosPlayed: [ '2H' ],
    });
  });

  it('derives latest one-off context from tokenlog regardless of current phase', () => {
    const tokenlog = [
      'V1 CUTTHROAT3P DEALER P0 DECK AC AD AH AS ENDDECK',
      'P0 oneOff 4C P2',
      'P1 resolve',
      'P2 resolve',
      'P0 resolve discard 7H',
    ].join(' ');

    expect(deriveLatestOneOffContextFromTokenlog(tokenlog)).toEqual({
      oneOffSeat: 0,
      oneOffCardToken: '4C',
      oneOffTarget: {
        type: 'Player',
        seat: 2,
      },
    });
  });

  it('derives phase counter context with one-off seat from base_player', () => {
    const phase = {
      type: 'Countering',
      data: {
        base_player: 2,
        oneoff: {
          type: 'PlayOneOff',
          data: {
            card: '4C',
            target: {
              type: 'Player',
              data: {
                seat: 1,
              },
            },
          },
        },
        twos: [ { seat: 0, card: '2H' } ],
      },
    };

    expect(deriveCounterDialogContextFromPhase(phase)).toEqual({
      oneOffSeat: 2,
      triggeringSeat: 0,
      oneOffCardToken: '4C',
      oneOffTarget: {
        type: 'Player',
        seat: 1,
      },
      twosPlayed: [ '2H' ],
    });
  });

  it('derives phase counter context with latest two seat as triggering seat', () => {
    const phase = {
      type: 'Countering',
      data: {
        base_player: 1,
        oneoff: {
          type: 'PlayOneOff',
          data: {
            card: '7D',
            target: {
              type: 'None',
              data: {},
            },
          },
        },
        twos: [
          { seat: 2, card: '2S' },
          { seat: 0, card: '2H' },
        ],
      },
    };

    expect(deriveCounterDialogContextFromPhase(phase)).toEqual({
      oneOffSeat: 1,
      triggeringSeat: 0,
      oneOffCardToken: '7D',
      oneOffTarget: {
        type: 'None',
      },
      twosPlayed: [ '2S', '2H' ],
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

  it('formats one-off resolution lines with counters and fizzles', () => {
    const resolves = [
      'V1 CUTTHROAT3P DEALER P0 DECK AC AD AH AS ENDDECK',
      'P1 oneOff 4C P2',
      'P2 resolve',
      'P2 resolve discard 7H',
      'P2 resolve discard 8D',
    ].join(' ');
    expect(formatTokenlogForHistory(resolves, { seatNames: { 0: 'Dealer', 1: 'Avi', 2: 'Spud' } })).toEqual([
      'Dealer dealt; Avi will go first',
      'Avi played the 4♣️ as a one-off to Your opponent discards two cards of their choice from their hand, targeting Spud.',
      'The 4♣️ one-off resolves; Spud must discard two cards.',
      'Spud discarded the 7♥️.',
      'Spud discarded the 8♦️.',
    ]);

    const fizzles = [
      'V1 CUTTHROAT3P DEALER P0 DECK AC AD AH AS ENDDECK',
      'P1 oneOff 6S',
      'P2 counter 2C',
    ].join(' ');
    expect(formatTokenlogForHistory(fizzles, { seatNames: { 0: 'Dealer', 1: 'Avi', 2: 'Spud' } })).toEqual([
      'Dealer dealt; Avi will go first',
      'Avi played the 6♠️ as a one-off to Scrap all Royals and Glasses eights.',
      'Spud played the 2♣️ to counter.',
      'The 6♠️ is countered, and all cards played this turn are scrapped.',
    ]);
  });

  it('formats resolve-five discard sequences and replay-scoped lines', () => {
    const tokenlog = [
      'V1 CUTTHROAT3P DEALER P2 DECK AC AD AH AS ENDDECK',
      'P0 oneOff 5H',
      'P1 resolve',
      'P2 resolve',
      'P0 discard 7D',
      'P0 draw UNKNOWN',
    ].join(' ');

    expect(formatTokenlogForHistory(tokenlog, { seatNames: { 0: 'Avi', 1: 'Bob', 2: 'Cy' } })).toEqual([
      'Cy dealt; Avi will go first',
      'Avi played the 5♥️ as a one-off to Discard 1 card, and draw up to 3.',
      'The 5♥️ one-off resolves; Avi must discard 1 card, and will draw up to 3.',
      'Avi discarded the 7♦️.',
      'Avi drew a card.',
    ]);

    expect(formatTokenlogForHistory(tokenlog, {
      seatNames: { 0: 'Avi', 1: 'Bob', 2: 'Cy' },
      maxActions: 1,
    })).toEqual([
      'Cy dealt; Avi will go first',
      'Avi played the 5♥️ as a one-off to Discard 1 card, and draw up to 3.',
      'The 5♥️ one-off resolves; Avi must discard 1 card, and will draw up to 3.',
    ]);
  });
});
