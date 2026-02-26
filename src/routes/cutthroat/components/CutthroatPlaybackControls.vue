<template>
  <nav
    id="playback-controls"
    data-cy="playback-controls"
    :aria-label="t('cutthroat.game.playbackControls')"
  >
    <span id="playback-controls-button-wrapper">
      <v-btn
        :disabled="!canGoToPreviousState"
        variant="text"
        icon="mdi-skip-backward"
        data-cy="skip-backward"
        :aria-label="t('cutthroat.game.playbackFirst')"
        :title="t('cutthroat.game.playbackFirst')"
        @click="goToState(0)"
      />

      <v-btn
        :disabled="!canGoToPreviousState"
        variant="text"
        icon="mdi-step-backward"
        data-cy="step-backward"
        :aria-label="t('cutthroat.game.playbackPrevious')"
        :title="t('cutthroat.game.playbackPrevious')"
        @click="goToState(previousGameStateIndex)"
      />

      <v-btn
        :disabled="!canGoToNextState"
        variant="text"
        icon="mdi-step-forward"
        data-cy="step-forward"
        :aria-label="t('cutthroat.game.playbackNext')"
        :title="t('cutthroat.game.playbackNext')"
        @click="goToState(currentGameStateIndex + 1)"
      />

      <v-btn
        :disabled="!canGoToNextState"
        variant="text"
        icon="mdi-skip-forward"
        data-cy="skip-forward"
        :aria-label="t('cutthroat.game.playbackLatest')"
        :title="t('cutthroat.game.playbackLatest')"
        @click="goToState(-1)"
      />
    </span>
  </nav>
</template>

<script setup>
import { computed } from 'vue';
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';

const props = defineProps({
  gameId: {
    type: Number,
    required: true,
  },
  stateCount: {
    type: Number,
    required: true,
  },
});

const { t } = useI18n();
const router = useRouter();
const currentGameStateIndex = computed(() => {
  const route = router.currentRoute.value;
  const index = Number(route.query.gameStateIndex);
  return Number.isInteger(index) && index >= -1 ? index : 0;
});

const canGoToPreviousState = computed(() => {
  return props.stateCount >= 2
    && (currentGameStateIndex.value === -1 || currentGameStateIndex.value > 0);
});

const canGoToNextState = computed(() => {
  return currentGameStateIndex.value >= 0 && currentGameStateIndex.value < props.stateCount - 1;
});

const previousGameStateIndex = computed(() => {
  return currentGameStateIndex.value === -1
    ? props.stateCount - 2
    : currentGameStateIndex.value - 1;
});

function goToState(gameStateIndex) {
  const route = router.currentRoute.value;
  router.push({
    ...route,
    query: {
      ...route.query,
      gameStateIndex,
    },
  });
}
</script>

<style scoped>
#playback-controls {
  position: fixed;
  display: flex;
  justify-content: center;
  align-items: center;
  bottom: 0;
  background-color: rgba(var(--v-theme-surface-1)) !important;
  width: 100%;
  z-index: 2411;
}

#playback-controls-button-wrapper {
  position: relative;
  display: flex;
  justify-content: center;
  align-items: center;
}
</style>
