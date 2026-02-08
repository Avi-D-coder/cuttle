import { defineStore } from 'pinia';
import { ref } from 'vue';
import { io } from '@/plugins/sails';
import { resolveCutthroatHttpPath } from '@/util/cutthroat-url';
import { useAuthStore } from '@/stores/auth';

export const useMyGamesStore = defineStore('myGames', () => {
  const authStore = useAuthStore();
  const games = ref([]);
  const loading = ref(false);
  const hasMore = ref(true);
  const totalCount = ref(0);

  const classicGames = ref([]);
  const cutthroatGames = ref([]);
  const classicSkip = ref(0);
  const classicHasMore = ref(true);
  const cutthroatHasMore = ref(true);
  const cutthroatCursor = ref(null);
  const cutthroatAvailable = ref(true);

  function getClassicWinnerLabel(game) {
    if (!game.winnerId) {return null;}
    const opponent = game.p0.isOpponent ? game.p0 : game.p1;
    return game.winnerId !== opponent.id;
  }

  function normalizeClassicGame(game) {
    const opponent = game.p0.isOpponent ? game.p0 : game.p1;
    return {
      id: `classic-${game.id}`,
      mode: 'classic',
      name: game.name,
      gameId: game.id,
      isRanked: game.isRanked,
      winnerLabel: getClassicWinnerLabel(game),
      opponentName: opponent.username,
      replayRoute: `/spectate/${game.id}`,
      playedAt: game.createdAt,
    };
  }

  function normalizeCutthroatGame(game) {
    const viewer = authStore.username;
    const opponents = (game.players ?? [])
      .filter((player) => player.username !== viewer)
      .map((player) => player.username);
    const opponentName = opponents.length > 0
      ? opponents.join(', ')
      : (game.players ?? []).map((player) => player.username).join(', ');
    return {
      id: `cutthroat-${game.rust_game_id}`,
      mode: 'cutthroat',
      name: game.name,
      gameId: game.rust_game_id,
      isRanked: false,
      winnerLabel: game.viewer_won === undefined ? null : game.viewer_won,
      opponentName,
      replayRoute: `/cutthroat/spectate/${game.rust_game_id}`,
      playedAt: game.finished_at,
    };
  }

  function comparePlayedAtDescending(left, right) {
    const leftTime = new Date(left.playedAt).getTime();
    const rightTime = new Date(right.playedAt).getTime();
    if (leftTime !== rightTime) {
      return rightTime - leftTime;
    }
    return right.id.localeCompare(left.id);
  }

  async function fetchClassicPage({ sortBy, sortDirection, limit }) {
    if (!classicHasMore.value) {
      return { games: [], hasMore: false };
    }
    const query = new URLSearchParams({
      sortBy,
      sortDirection,
      limit: limit.toString(),
      skip: classicSkip.value.toString(),
    });

    const { finishedGames, hasMore: moreAvailable } = await new Promise((resolve, reject) => {
      io.socket.get(`/api/game/history?${query.toString()}`, (res, jwres) => {
        if (jwres?.statusCode === 200 && Array.isArray(res.finishedGames)) {
          resolve(res);
        } else {
          reject(new Error('Failed to load 2P game history'));
        }
      });
    });

    const normalizedGames = finishedGames.map(normalizeClassicGame);
    return {
      games: normalizedGames,
      hasMore: Boolean(moreAvailable),
    };
  }

  async function fetchCutthroatPage(limit) {
    if (!cutthroatAvailable.value || !cutthroatHasMore.value) {
      return { games: [], hasMore: false, nextCursor: null };
    }

    const query = new URLSearchParams({
      limit: limit.toString(),
    });
    if (cutthroatCursor.value?.before_finished_at && cutthroatCursor.value?.before_rust_game_id) {
      query.set('before_finished_at', cutthroatCursor.value.before_finished_at);
      query.set('before_rust_game_id', String(cutthroatCursor.value.before_rust_game_id));
    }

    const response = await fetch(resolveCutthroatHttpPath(`/cutthroat/api/v1/history?${query.toString()}`), {
      method: 'GET',
      credentials: 'include',
    });
    if (!response.ok) {
      if (response.status >= 500 || response.status === 404) {
        cutthroatAvailable.value = false;
        return { games: [], hasMore: false, nextCursor: null };
      }
      throw new Error(`Failed to load 3P game history: ${response.status}`);
    }
    const payload = await response.json();
    const normalizedGames = Array.isArray(payload.finished_games)
      ? payload.finished_games.map(normalizeCutthroatGame)
      : [];
    return {
      games: normalizedGames,
      hasMore: Boolean(payload.has_more),
      nextCursor: payload.next_cursor ?? null,
    };
  }

  function rebuildMergedGames() {
    games.value = [ ...classicGames.value, ...cutthroatGames.value ]
      .sort(comparePlayedAtDescending);
    totalCount.value = games.value.length;
    hasMore.value = classicHasMore.value || (cutthroatAvailable.value && cutthroatHasMore.value);
  }

  async function loadMyGames(options = {}) {
    if (loading.value) {return;}

    loading.value = true;
    try {
      const {
        sortBy = 'createdAt',
        sortDirection = 'desc',
        limit = 20,
        reset = false
      } = options;

      if (reset) {
        classicGames.value = [];
        cutthroatGames.value = [];
        classicSkip.value = 0;
        classicHasMore.value = true;
        cutthroatHasMore.value = true;
        cutthroatCursor.value = null;
        cutthroatAvailable.value = true;
      } else if (!hasMore.value) {
        return;
      }

      const [ classicPage, cutthroatPage ] = await Promise.all([
        fetchClassicPage({ sortBy, sortDirection, limit }),
        fetchCutthroatPage(limit),
      ]);

      if (classicPage.games.length > 0) {
        classicGames.value = [ ...classicGames.value, ...classicPage.games ];
        classicSkip.value += classicPage.games.length;
      }
      classicHasMore.value = classicPage.hasMore;

      if (cutthroatPage.games.length > 0) {
        cutthroatGames.value = [ ...cutthroatGames.value, ...cutthroatPage.games ];
      }
      cutthroatHasMore.value = cutthroatPage.hasMore;
      cutthroatCursor.value = cutthroatPage.nextCursor;

      rebuildMergedGames();
    } catch (err) {
      console.error('Error loading games:', err);
    } finally {
      loading.value = false;
    }
  }

  function resetGames() {
    loading.value = false;
    games.value = [];
    classicGames.value = [];
    cutthroatGames.value = [];
    classicSkip.value = 0;
    classicHasMore.value = true;
    cutthroatHasMore.value = true;
    cutthroatCursor.value = null;
    cutthroatAvailable.value = true;
    hasMore.value = true;
    totalCount.value = 0;
  }

  return {
    games,
    loading,
    hasMore,
    totalCount,
    loadMyGames,
    resetGames,
  };
});
