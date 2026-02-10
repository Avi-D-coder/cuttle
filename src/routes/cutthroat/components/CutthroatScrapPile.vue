<template>
  <BaseDialog
    id="cutthroat-scrap-dialog"
    v-model="showDialog"
    :scrollable="true"
    :persistent="false"
    :attach="false"
  >
    <template #activator="{ props: dialogProps }">
      <span
        v-bind="dialogProps"
        @click="openDialog"
        @mousedown="isLongPressing = false"
        @touchstart="isLongPressing = false"
      >
        <div
          id="cutthroat-scrap"
          ref="scrapWrapper"
          class="d-flex flex-column align-center"
          :class="{ 'is-straightened': isStraightened }"
          data-cy="cutthroat-scrap"
        >
          <TransitionGroup name="scrap">
            <GameCard
              v-for="(card, index) in scrapDisplay"
              :key="card.id"
              :suit="card.suit"
              :rank="card.rank"
              :custom-elevation="index > straightenedIndex ? index : 0"
              class="position-absolute scrap-card"
              :class="index > straightenedIndex ? `scrap-card-${index % 10}` : ''"
            >
              <template v-if="index === scrapDisplay.length - 1" #overlay>
                <v-overlay
                  :model-value="true"
                  contained
                  persistent
                  scrim="surface-1"
                  opacity=".46"
                  class="d-flex flex-column justify-space-around align-center rounded-lg"
                  content-class="d-flex flex-column align-center"
                >
                  <h3 id="scrap-header">{{ t('cutthroat.game.scrap') }}</h3>
                  <p id="scrap-length" class="text-surface-2 text-center mb-4 mt-1">({{ scrapCards.length }})</p>
                  <v-btn variant="outlined" color="surface-2">
                    {{ t('game.view') }}
                  </v-btn>
                </v-overlay>
              </template>
            </GameCard>
          </TransitionGroup>

          <Transition name="scrap-empty">
            <div v-if="scrapCards.length === 0" id="empty-scrap-activator">
              <h3 id="scrap-header">{{ t('cutthroat.game.scrap') }}</h3>
              <p class="text-surface-2 text-center mb-4 mt-1">({{ scrapCards.length }})</p>
              <v-btn variant="outlined" color="surface-2">
                {{ t('game.view') }}
              </v-btn>
            </div>
          </Transition>
        </div>
      </span>
    </template>

    <template #title>
      <div class="d-flex justify-space-between align-center w-100">
        <h1>{{ t('game.dialogs.scrapDialog.scrapPile') }}</h1>
        <v-btn
          icon
          color="surface-2"
          variant="text"
          data-cy="close-cutthroat-scrap-dialog-x"
          aria-label="Close scrap dialog"
          @click="showDialog = false"
        >
          <v-icon icon="mdi-close" size="large" aria-hidden="true" />
        </v-btn>
      </div>
    </template>

    <template #body>
      <div class="mt-4">
        <CardListSortable
          :cards="dialogScrapCards"
          :empty-text="t('game.dialogs.scrapDialog.noCards')"
          data-selector-prefix="scrap-dialog"
          :selected-ids="selectedScrapCardIds"
          @select-card="handleSortedCardSelect"
        />
      </div>
    </template>

    <template #actions>
      <v-btn
        v-if="isResolvingThreeTurn"
        data-cy="cutthroat-three-resolve"
        color="surface-2"
        variant="flat"
        :disabled="selectedScrapToken === null"
        @click="confirmResolveThreePick"
      >
        {{ t('game.resolve') }}
      </v-btn>
      <v-btn
        data-cy="close-cutthroat-scrap-dialog-button"
        color="surface-1"
        variant="flat"
        @click="showDialog = false"
      >
        {{ t('global.close') }}
      </v-btn>
    </template>
  </BaseDialog>
</template>

<script setup>
import { computed, ref, watch } from 'vue';
import { onLongPress } from '@vueuse/core';
import { useI18n } from 'vue-i18n';
import { orderBy } from 'lodash';
import BaseDialog from '@/components/BaseDialog.vue';
import CardListSortable from '@/routes/game/components/CardListSortable.vue';
import GameCard from '@/routes/game/components/GameCard.vue';
import {
  isPlayableScrapToken,
  mapScrapEntriesToCards,
} from '@/routes/cutthroat/cutthroat-scrap-helpers';

const props = defineProps({
  scrapTokens: {
    type: Array,
    required: true,
  },
  isResolvingThreeTurn: {
    type: Boolean,
    default: false,
  },
  isActionDisabled: {
    type: Boolean,
    default: false,
  },
  isStraightened: {
    type: Boolean,
    default: false,
  },
});

const emit = defineEmits([ 'pick-scrap-card', 'request-scrap-straighten' ]);

const { t } = useI18n();

const showDialog = ref(false);
const hasAutoOpenedResolveThreeDialog = ref(false);
const selectedScrapToken = ref(null);

const scrapCards = computed(() => mapScrapEntriesToCards(props.scrapTokens));

const scrapDisplay = computed(() => scrapCards.value.slice(-10));
const straightenedIndex = computed(() => (props.isStraightened ? scrapDisplay.value.length - 1 : -1));

const sortedScrapCards = computed(() => {
  return orderBy(scrapCards.value, [ 'rank', 'suit' ]);
});

const dialogScrapCards = computed(() => {
  if (!props.isResolvingThreeTurn) {return sortedScrapCards.value;}
  return sortedScrapCards.value.filter((card) => card.rank !== 3);
});

const selectedScrapCardIds = computed(() => {
  if (!selectedScrapToken.value) {return [];}
  const selected = dialogScrapCards.value.find((card) => card.token === selectedScrapToken.value);
  return selected ? [ selected.id ] : [];
});

const scrapWrapper = ref(null);
const isLongPressing = ref(false);

onLongPress(scrapWrapper, () => {
  if (!scrapCards.value.length) {return;}
  isLongPressing.value = true;
  emit('request-scrap-straighten');
}, {
  stop: true,
});

function handleSortedCardSelect(card) {
  if (!props.isResolvingThreeTurn || props.isActionDisabled) {return;}
  const token = card?.token ?? null;
  if (!isPlayableScrapToken(token)) {return;}
  if (selectedScrapToken.value === token) {
    selectedScrapToken.value = null;
    return;
  }
  selectedScrapToken.value = token;
}

function confirmResolveThreePick() {
  const token = selectedScrapToken.value;
  if (!isPlayableScrapToken(token) || !props.isResolvingThreeTurn || props.isActionDisabled) {return;}
  emit('pick-scrap-card', token);
  selectedScrapToken.value = null;
  showDialog.value = false;
}

function openDialog(event) {
  if (!isLongPressing.value) {return;}
  event.preventDefault();
  event.stopPropagation();
  isLongPressing.value = false;
}

watch(
  () => props.isResolvingThreeTurn,
  (isResolvingThreeTurn) => {
    if (!isResolvingThreeTurn) {
      hasAutoOpenedResolveThreeDialog.value = false;
      selectedScrapToken.value = null;
      showDialog.value = false;
      return;
    }

    if (props.isActionDisabled || hasAutoOpenedResolveThreeDialog.value) {return;}

    showDialog.value = true;
    hasAutoOpenedResolveThreeDialog.value = true;
  },
);

watch(showDialog, (isOpen) => {
  if (!isOpen) {
    selectedScrapToken.value = null;
  }
});
</script>

<style scoped lang="scss">
#cutthroat-scrap {
  position: relative;
  margin: 8px;
  height: clamp(148px, 24vh, 228px);
  width: calc(clamp(148px, 24vh, 228px) / 1.3);
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  cursor: pointer;
  user-select: none;

  & #scrap-header {
    font-family: 'Luckiest Guy';
    color: rgba(var(--v-theme-surface-2));
    text-align: center;
    font-size: clamp(24px, 4vh, 40px);
    line-height: 1;
  }

  & #scrap-length {
    font-family: 'Luckiest Guy';
  }

  & #empty-scrap-activator {
    height: 100%;
    width: 100%;
    background-color: rgba(var(--v-theme-surface-1), 0.46);
    padding: 16px;
    border-radius: 20px;
    display: flex;
    flex-direction: column;
    justify-content: space-around;
    align-items: center;
  }

  & .scrap-card {
    transition: transform 0.3s ease-in;
  }

  @for $i from 0 through 10 {
    & .scrap-card-#{$i} {
      $rotation: sin($i * 30) * 8deg;
      $translateX: cos($i * 45) * 8px;
      $translateY: sin($i * 60) * 5px;

      transform: translate($translateX, $translateY) rotate($rotation);
    }
  }
}

#cutthroat-scrap .scrap-card.scrap-enter-active {
  transition: all 0.8s ease-out;
}

#cutthroat-scrap .scrap-card.scrap-enter-from {
  opacity: 0;
  transform: rotate(0deg) translateX(100px);
}

@media (max-width: 1280px) {
  #cutthroat-scrap {
    height: clamp(124px, 20vh, 184px);
    width: calc(clamp(124px, 20vh, 184px) / 1.3);
    margin: 4px;

    & #scrap-header {
      font-size: clamp(20px, 3.2vh, 30px);
    }

    & #empty-scrap-activator {
      padding: 10px;
      border-radius: 14px;
    }
  }
}

@media (max-width: 960px) {
  #cutthroat-scrap {
    height: clamp(96px, 16vh, 138px);
    width: calc(clamp(96px, 16vh, 138px) / 1.3);
    margin: 0;

    & #scrap-header {
      font-size: clamp(16px, 2.4vh, 22px);
    }

    & #empty-scrap-activator {
      padding: 8px;
      border-radius: 12px;
    }
  }
}

@media (max-width: 600px) {
  #cutthroat-scrap {
    height: 78px;
    width: 58px;
    margin: 0;

    & #scrap-header {
      font-size: 18px;
      line-height: 18px;
    }

    & #empty-scrap-activator {
      padding: 6px;
      border-radius: 10px;
    }
  }
}
</style>
