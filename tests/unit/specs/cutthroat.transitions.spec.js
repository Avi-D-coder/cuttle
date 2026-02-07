import { describe, expect, it } from 'vitest';
import {
  pointStackTransitionForSeat,
  royalStackTransitionForSeat,
  scuttledByTokenForPoint,
} from '@/routes/cutthroat/helpers/transitions';

describe('cutthroat transition helpers', () => {
  it('returns default transitions when no event is present', () => {
    expect(pointStackTransitionForSeat(null, 0, 0)).toBe('in-below-out-left');
    expect(pointStackTransitionForSeat(null, 1, 0)).toBe('in-above-out-below');
    expect(royalStackTransitionForSeat(null, 0, 0)).toBe('in-below-out-left');
    expect(royalStackTransitionForSeat(null, 1, 0)).toBe('in-above-out-below');
  });

  it('returns special point transitions for jack/resolve events', () => {
    expect(pointStackTransitionForSeat({ change: 'jack' }, 0, 0)).toBe('slide-above');
    expect(pointStackTransitionForSeat({ change: 'sevenJack' }, 1, 0)).toBe('slide-below');
    expect(pointStackTransitionForSeat({ change: 'resolve', oneoff_rank: 2 }, 0, 0)).toBe('slide-above');
    expect(pointStackTransitionForSeat({ change: 'resolve', oneoff_rank: 9, target_type: 'point' }, 0, 0)).toBe('slide-below');
  });

  it('returns special royal transitions for nine resolve on royal/joker', () => {
    expect(royalStackTransitionForSeat({ change: 'resolve', oneoff_rank: 9, target_type: 'royal' }, 0, 0)).toBe('slide-below');
    expect(royalStackTransitionForSeat({ change: 'resolve', oneoff_rank: 9, target_type: 'joker' }, 1, 0)).toBe('slide-above');
  });

  it('returns scuttled source token only when target matches a point scuttle event', () => {
    const lastEvent = {
      change: 'scuttle',
      target_type: 'point',
      target_token: '7C',
      source_token: '9D',
    };
    expect(scuttledByTokenForPoint(lastEvent, '7C')).toBe('9D');
    expect(scuttledByTokenForPoint(lastEvent, '8C')).toBeNull();
    expect(scuttledByTokenForPoint({ change: 'resolve' }, '7C')).toBeNull();
  });
});
