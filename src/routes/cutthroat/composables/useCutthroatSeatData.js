import { computed } from 'vue';
import {
  parseCardToken,
  publicCardToDisplay,
  publicCardToken,
} from '@/util/cutthroat-cards';
import { makeSeatLabel } from '@/routes/cutthroat/helpers/game-state';

export function useCutthroatSeatData({
  playerView,
  phaseData,
  isResolvingSeven,
  mySeat,
  seatEntries,
  localHandActionTokens,
}) {
  const leftSeat = computed(() => (mySeat.value + 1) % 3);
  const rightSeat = computed(() => (mySeat.value + 2) % 3);
  const loggedHydrationMismatches = new Set();

  function logHydrationMismatchOnce(seat, visibleTokens, inferredTokens) {
    const sortedVisible = visibleTokens.slice().sort();
    const sortedInferred = inferredTokens.slice().sort();
    const key = `${seat}|${sortedVisible.join(',')}|${sortedInferred.join(',')}`;
    if (loggedHydrationMismatches.has(key)) {return;}
    loggedHydrationMismatches.add(key);
    console.error(
      '[cutthroat] hand token mismatch: legal actions referenced hand tokens not present in visible hand',
      {
        seat,
        visibleTokens,
        inferredTokens,
      },
    );
  }

  function playerForSeat(seat) {
    return playerView.value?.players?.find((player) => player.seat === seat);
  }

  function handFor(seat) {
    const player = playerForSeat(seat);
    if (!player) {return [];}
    const entries = player.hand.map((card, index) => {
      const token = publicCardToken(card);
      return {
        token,
        key: token ?? `hidden-${seat}-${index}`,
        card: publicCardToDisplay(card),
        isKnown: Boolean(token),
      };
    });

    if (seat !== mySeat.value) {
      return entries;
    }

    const knownTokens = new Set(entries.filter((entry) => entry.isKnown && entry.token).map((entry) => entry.token));
    const extraTokens = localHandActionTokens.value.filter((token) => !knownTokens.has(token));
    if (extraTokens.length > 0) {
      const visibleTokens = entries.filter((entry) => entry.isKnown && entry.token).map((entry) => entry.token);
      logHydrationMismatchOnce(seat, visibleTokens, extraTokens);
    }
    return entries;
  }

  function frozenFor(seat) {
    const player = playerForSeat(seat);
    if (!player) {return [];}
    return (player.frozen ?? []).map((token, index) => ({
      token,
      key: `${token}-${index}`,
      card: parseCardToken(token),
    }));
  }

  function pointsFor(seat) {
    const player = playerForSeat(seat);
    if (!player) {return [];}
    return player.points.map((stack) => ({
      baseToken: stack.base,
      baseCard: parseCardToken(stack.base),
      controller: stack.controller,
      attachments: stack.jacks.map((token) => ({
        token,
        card: parseCardToken(token),
      })),
    }));
  }

  function royalsFor(seat) {
    const player = playerForSeat(seat);
    if (!player) {return [];}
    return player.royals.map((stack) => ({
      baseToken: stack.base,
      baseCard: parseCardToken(stack.base),
      isGlasses: stack.base?.startsWith('8'),
      controller: stack.controller,
      attachments: stack.jokers.map((token) => ({
        token,
        card: parseCardToken(token),
      })),
    }));
  }

  function seatLabel(seat) {
    return makeSeatLabel(seat, seatEntries.value);
  }

  const myFrozenTokens = computed(() => {
    const player = playerForSeat(mySeat.value);
    return new Set(player?.frozen ?? []);
  });

  const revealedCardEntries = computed(() => {
    if (!isResolvingSeven.value) {return [];}
    return (phaseData.value?.revealed_cards ?? []).map((token, index) => ({
      token,
      index,
      key: `reveal-${index}-${token}`,
      card: parseCardToken(token),
    }));
  });

  const leftHandCards = computed(() => handFor(leftSeat.value));
  const rightHandCards = computed(() => handFor(rightSeat.value));
  const myHandCards = computed(() => handFor(mySeat.value));

  const leftPointStacks = computed(() => pointsFor(leftSeat.value));
  const rightPointStacks = computed(() => pointsFor(rightSeat.value));
  const myPointStacks = computed(() => pointsFor(mySeat.value));

  const leftRoyalStacks = computed(() => royalsFor(leftSeat.value));
  const rightRoyalStacks = computed(() => royalsFor(rightSeat.value));
  const myRoyalStacks = computed(() => royalsFor(mySeat.value));

  const myFrozenCards = computed(() => frozenFor(mySeat.value));

  return {
    leftSeat,
    rightSeat,
    myFrozenTokens,
    revealedCardEntries,
    leftHandCards,
    rightHandCards,
    myHandCards,
    leftPointStacks,
    rightPointStacks,
    myPointStacks,
    leftRoyalStacks,
    rightRoyalStacks,
    myRoyalStacks,
    myFrozenCards,
    seatLabel,
  };
}
