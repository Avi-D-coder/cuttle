import { describe, expect, it } from 'vitest';
import {
  getCutthroatGameResult,
  isActionInteractionDisabled,
  isCutthroatGameFinished,
  makeSeatLabel,
  shouldShowCutthroatGameOverDialog,
  shouldRedirectToCutthroatGame,
} from '@/routes/cutthroat/helpers/game-state';

describe('cutthroat game-state helpers', () => {
  it('redirects to game when status is started', () => {
    expect(shouldRedirectToCutthroatGame(1)).toBe(true);
    expect(shouldRedirectToCutthroatGame(0)).toBe(false);
    expect(shouldRedirectToCutthroatGame(2)).toBe(false);
  });

  it('marks finished status and disables action interactions', () => {
    expect(isCutthroatGameFinished(2)).toBe(true);
    expect(isActionInteractionDisabled(2, false)).toBe(true);
    expect(isActionInteractionDisabled(1, true)).toBe(true);
    expect(isActionInteractionDisabled(1, false)).toBe(false);
  });

  it('builds seat labels with usernames when available', () => {
    const seats = [
      { seat: 0, username: 'avi' },
      { seat: 1, username: '' },
    ];
    expect(makeSeatLabel(0, seats)).toBe('avi');
    expect(makeSeatLabel(1, seats)).toBe('Player 2');
    expect(makeSeatLabel(2, seats)).toBe('Player 3');
  });

  it('derives winner and draw from finished public view', () => {
    const winnerView = {
      players: [
        {
          seat: 0,
          points: [ { base: 'KC' }, { base: 'AH' } ],
          royals: [ { base: 'KH' } ],
        },
        {
          seat: 1,
          points: [],
          royals: [],
        },
      ],
    };

    expect(getCutthroatGameResult(2, winnerView)).toEqual({
      type: 'winner',
      seat: 0,
    });

    const drawView = {
      players: [
        { seat: 0, points: [], royals: [] },
        { seat: 1, points: [ { base: '4C' } ], royals: [] },
      ],
    };

    expect(getCutthroatGameResult(2, drawView)).toEqual({
      type: 'draw',
      seat: null,
    });
  });

  it('shows game over dialog only at replay end when spectating', () => {
    expect(shouldShowCutthroatGameOverDialog({
      status: 2,
      isSpectateRoute: false,
      hasReplayStateIndexQuery: false,
      replayStateIndex: -1,
      replayStateCount: 10,
    })).toBe(true);

    expect(shouldShowCutthroatGameOverDialog({
      status: 2,
      isSpectateRoute: true,
      hasReplayStateIndexQuery: false,
      replayStateIndex: -1,
      replayStateCount: 10,
    })).toBe(false);

    expect(shouldShowCutthroatGameOverDialog({
      status: 2,
      isSpectateRoute: true,
      hasReplayStateIndexQuery: true,
      replayStateIndex: 0,
      replayStateCount: 10,
    })).toBe(false);

    expect(shouldShowCutthroatGameOverDialog({
      status: 2,
      isSpectateRoute: true,
      hasReplayStateIndexQuery: true,
      replayStateIndex: 9,
      replayStateCount: 10,
    })).toBe(true);

    expect(shouldShowCutthroatGameOverDialog({
      status: 2,
      isSpectateRoute: true,
      hasReplayStateIndexQuery: true,
      replayStateIndex: -1,
      replayStateCount: 10,
    })).toBe(true);
  });
});
