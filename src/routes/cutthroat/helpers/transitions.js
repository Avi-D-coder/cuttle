function defaultPointTransition(isLocalSeat) {
  return isLocalSeat ? 'in-below-out-left' : 'in-above-out-below';
}

function defaultRoyalTransition(isLocalSeat) {
  return isLocalSeat ? 'in-below-out-left' : 'in-above-out-below';
}

function isSameSeat(seatA, seatB) {
  return typeof seatA === 'number' && typeof seatB === 'number' && seatA === seatB;
}

export function pointStackTransitionForSeat(lastEvent, seat, mySeat) {
  const isLocalSeat = isSameSeat(seat, mySeat);
  if (!lastEvent?.change) {
    return defaultPointTransition(isLocalSeat);
  }

  switch (lastEvent.change) {
    case 'jack':
    case 'sevenJack':
      return isLocalSeat ? 'slide-above' : 'slide-below';
    case 'resolve':
      switch (lastEvent.oneoff_rank) {
        case 2:
        case 6:
          return isLocalSeat ? 'slide-above' : 'slide-below';
        case 9:
          if (lastEvent.target_type === 'jack') {
            return isLocalSeat ? 'slide-above' : 'slide-below';
          }
          return isLocalSeat ? 'slide-below' : 'slide-above';
        default:
          return defaultPointTransition(isLocalSeat);
      }
    default:
      return defaultPointTransition(isLocalSeat);
  }
}

export function royalStackTransitionForSeat(lastEvent, seat, mySeat) {
  const isLocalSeat = isSameSeat(seat, mySeat);
  if (
    lastEvent?.change === 'resolve'
    && lastEvent?.oneoff_rank === 9
    && (lastEvent?.target_type === 'royal' || lastEvent?.target_type === 'joker')
  ) {
    return isLocalSeat ? 'slide-below' : 'slide-above';
  }
  return defaultRoyalTransition(isLocalSeat);
}

export function scuttledByTokenForPoint(lastEvent, pointBaseToken) {
  if (!lastEvent || lastEvent.change !== 'scuttle') {
    return null;
  }
  if (lastEvent.target_type !== 'point') {
    return null;
  }
  if (lastEvent.target_token !== pointBaseToken) {
    return null;
  }
  return lastEvent.source_token ?? null;
}
