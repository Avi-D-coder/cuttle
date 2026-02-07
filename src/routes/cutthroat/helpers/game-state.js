function rankFromToken(token = '') {
  const [ rankChar ] = token;
  if (!rankChar) {return 0;}
  const mapped = {
    A: 1,
    T: 10,
    J: 11,
    Q: 12,
    K: 13,
  }[rankChar];
  if (mapped) {return mapped;}
  const parsed = Number(rankChar);
  return Number.isFinite(parsed) ? parsed : 0;
}

function pointsToWinByKings(kings) {
  if (kings >= 3) {return 0;}
  if (kings === 2) {return 5;}
  if (kings === 1) {return 9;}
  return 14;
}

function kingCount(player) {
  return (player?.royals ?? []).filter((stack) => {
    return rankFromToken(stack?.base ?? '') === 13;
  }).length;
}

function pointTotal(player) {
  return (player?.points ?? []).reduce((total, stack) => {
    return total + rankFromToken(stack?.base ?? '');
  }, 0);
}

export function shouldRedirectToCutthroatGame(status) {
  return status === 1;
}

export function isCutthroatGameFinished(status) {
  return status === 2;
}

export function isActionInteractionDisabled(status, actionInFlight, isSpectator = false) {
  return isCutthroatGameFinished(status) || actionInFlight || isSpectator;
}

export function getCutthroatGameResult(status, publicView) {
  if (!isCutthroatGameFinished(status)) {
    return {
      type: 'in_progress',
      seat: null,
    };
  }
  const players = publicView?.players ?? [];
  const winners = players
    .filter((player) => {
      return pointTotal(player) >= pointsToWinByKings(kingCount(player));
    })
    .map((player) => player.seat);

  if (winners.length === 1) {
    const [ winnerSeat ] = winners;
    return {
      type: 'winner',
      seat: winnerSeat,
    };
  }

  return {
    type: 'draw',
    seat: null,
  };
}

export function makeSeatLabel(seat, seats = []) {
  const found = seats.find((entry) => entry.seat === seat);
  const username = found?.username?.trim();
  if (username) {
    return username;
  }
  return `Player ${seat + 1}`;
}
