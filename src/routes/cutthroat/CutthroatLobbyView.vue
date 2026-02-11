<template>
  <div id="cutthroat-lobby-wrapper">
    <div class="langauge-selector">
      <TheLanguageSelector variant="light" />
    </div>
    <v-container>
      <div class="d-flex align-center">
        <h1>{{ t('cutthroat.lobby.title') }}</h1>
      </div>
      <h5 v-if="lobbyName">
        {{ lobbyName }}
      </h5>

      <v-row>
        <v-col
          v-for="seat in seatViews"
          :key="seat.seat"
          cols="12"
          md="4"
        >
          <div class="seat-label text-overline">
            {{ t('cutthroat.lobby.seat') }} {{ seat.seat + 1 }}
          </div>
          <PlayerReadyIndicator
            :player-username="seat.username || null"
            :player-ready="seat.ready"
            data-cy="cutthroat-seat-indicator"
          />
        </v-col>
      </v-row>

      <v-row>
        <v-spacer />
        <v-col class="home-card-games" :cols="$vuetify.display.mdAndUp ? 8 : 12">
          <div class="mx-auto my-4 my-xl-2 homeContent">
            <v-btn
              class="px-16 w-100"
              color="primary"
              size="x-large"
              text-color="white"
              data-cy="cutthroat-ready-button"
              :disabled="!mySeatAssigned || readying"
              @click="toggleReady"
            >
              {{ myReady ? t('cutthroat.lobby.unready') : t('cutthroat.lobby.readyUp') }}
            </v-btn>
            <div class="d-flex flex-row justify-md-space-between justify-space-evenly align-center flex-wrap my-4">
              <p class="mb-0 cutthroat-auto-start">
                {{ t('cutthroat.lobby.autoStart') }}
              </p>
              <v-btn
                variant="text"
                class="w-50 px-16 py-2"
                color="surface-2"
                data-cy="cutthroat-exit-button"
                size="x-large"
                @click="leaveLobby"
              >
                {{ t('lobby.exit') }}
              </v-btn>
            </div>
          </div>
        </v-col>
        <v-spacer />
      </v-row>
    </v-container>
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { useCutthroatStore } from '@/stores/cutthroat';
import { useSnackbarStore } from '@/stores/snackbar';
import { shouldRedirectToCutthroatGame } from '@/routes/cutthroat/helpers/game-state';
import PlayerReadyIndicator from '@/components/PlayerReadyIndicator.vue';
import TheLanguageSelector from '@/components/TheLanguageSelector.vue';

const route = useRoute();
const router = useRouter();
const { t } = useI18n();
const store = useCutthroatStore();
const snackbarStore = useSnackbarStore();
const readying = ref(false);

const gameId = computed(() => Number(route.params.gameId));
const lobbyName = computed(() => {
  return store.lobby?.name || '';
});

const seatViews = computed(() => {
  const seats = store.lobby?.seats ?? [];
  return [ 0, 1, 2 ].map((seat) => {
    const found = seats.find((entry) => entry.seat === seat);
    return {
      seat,
      username: found?.username ?? '',
      ready: found?.ready ?? false,
      user_id: found?.user_id ?? null,
    };
  });
});

const mySeatAssigned = computed(() => store.seat !== null && store.seat !== undefined);
const myReady = computed(() => {
  const entry = seatViews.value.find((seat) => seat.seat === store.seat);
  return entry?.ready ?? false;
});

async function ensureJoined() {
  try {
    await store.fetchState(gameId.value);
    if (shouldRedirectToCutthroatGame(store.status)) {
      router.replace(`/cutthroat/game/${gameId.value}`);
      return;
    }
    store.connectWs(gameId.value);
    return;
  } catch (err) {
    if (err?.status !== 403 && err?.status !== 404 && err?.status !== 409) {
      snackbarStore.alert(err?.message ?? t('cutthroat.lobby.joinFailed'));
      router.push('/');
      return;
    }
  }

  try {
    await store.joinGame(gameId.value);
    await store.fetchState(gameId.value);
    if (shouldRedirectToCutthroatGame(store.status)) {
      router.replace(`/cutthroat/game/${gameId.value}`);
      return;
    }
    store.connectWs(gameId.value);
  } catch (err) {
    snackbarStore.alert(err?.message ?? t('cutthroat.lobby.joinFailed'));
    router.push('/');
  }
}

async function toggleReady() {
  readying.value = true;
  try {
    await store.setReady(gameId.value, !myReady.value);
  } catch (err) {
    snackbarStore.alert(err?.message ?? t('cutthroat.lobby.readyFailed'));
  } finally {
    readying.value = false;
  }
}

function leaveLobby() {
  store.leaveGame(gameId.value)
    .catch(() => {})
    .finally(() => {
      router.push('/');
    });
}

watch(
  () => store.status,
  (status) => {
    if (shouldRedirectToCutthroatGame(status)) {
      router.replace(`/cutthroat/game/${gameId.value}`);
    }
  },
);

watch(
  () => store.lastError,
  (error) => {
    if (!error) {return;}
    snackbarStore.alert(error.message ?? t('cutthroat.lobby.readyFailed'));
    store.clearLastError();
  },
);

watch(
  () => gameId.value,
  (id) => {
    if (!id) {
      router.push('/');
    }
  },
);

onMounted(async () => {
  await ensureJoined();
});

onBeforeUnmount(() => {
  store.disconnectWs();
});
</script>

<style scoped lang="scss">
#cutthroat-lobby-wrapper {
  color: rgba(var(--v-theme-surface-2));
  min-width: 100vw;
  min-height: 100vh;
  text-align: center;
  background: rgba(var(--v-theme-surface-1));
  box-shadow: inset 0 0 700px -1px #000000;
}

h1 {
  font-size: 5rem;
  color: rgba(var(--v-theme-surface-2));
  font-family: 'Luckiest Guy', serif !important;
  font-weight: 400;
  line-height: 5rem;
  margin: auto auto 16px auto;
}

h5 {
  font-size: 3rem;
  color: rgba(var(--v-theme-surface-2));
  font-family: 'Luckiest Guy', serif !important;
  font-weight: 400;
  line-height: 5rem;
  margin: auto auto 16px auto;
}

.langauge-selector {
  position: absolute;
  right: 0;
  top: 20px;
  width: min-content;
}

.seat-label {
  margin-bottom: 8px;
}

.home-card-games {
  padding: 0;
  max-width: 640px;
  margin: 0 auto;
}

.homeContent {
  max-width: 580px;
}

.cutthroat-auto-start {
  width: 50%;
  color: rgba(var(--v-theme-surface-2));
  text-align: left;
}

@media (max-width: 660px) {
  h1 {
    font-size: 2rem;
    line-height: 2rem;
    margin: 0 auto;
  }

  h5 {
    font-size: 2rem;
    line-height: 2rem;
    margin: 0 auto 16px auto;
  }

  .cutthroat-auto-start {
    width: 100%;
    text-align: center;
    margin-bottom: 8px;
  }
}
</style>
