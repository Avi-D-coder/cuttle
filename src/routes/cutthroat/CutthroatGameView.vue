<template>
  <div id="cutthroat-game-wrapper">
    <div v-if="!playerView" class="loading">
      {{ t('cutthroat.game.loading') }}
    </div>

    <template v-else>
      <div v-if="smAndDown" class="mobile-history-controls">
        <v-icon
          class="history-toggle-icon"
          color="white"
          icon="mdi-account-clock"
          size="large"
          aria-label="Show game history"
          aria-hidden="false"
          role="button"
          data-cy="cutthroat-history-toggle"
          @click.stop="showHistoryDrawer = !showHistoryDrawer"
        />
      </div>

      <v-navigation-drawer
        v-if="smAndDown"
        v-model="showHistoryDrawer"
        class="c-history-drawer"
        location="right"
      >
        <template #prepend>
          <v-list-item>
            <h3>{{ t('game.history.title') }}</h3>
            <template #append>
              <v-btn icon variant="text" @click.stop="showHistoryDrawer = !showHistoryDrawer">
                <v-icon
                  color="neutral"
                  icon="mdi-window-close"
                  size="large"
                  aria-label="window close icon"
                  aria-hidden="false"
                  role="img"
                />
              </v-btn>
            </template>
          </v-list-item>
        </template>

        <v-divider />

        <div ref="logsContainerDrawer" class="history-logs history-logs-drawer">
          <p
            v-for="(log, index) in historyLines"
            :key="`cutthroat-log-drawer-${index}`"
            class="history-log"
            data-cy="cutthroat-history-log"
            data-cy-history-log="history-log"
          >
            {{ log }}
          </p>
          <p v-if="historyLines.length === 0" class="history-log history-log-empty">
            {{ t('cutthroat.game.noActions') }}
          </p>
        </div>
      </v-navigation-drawer>

      <div
        class="table"
        :class="{ 'compact-resolving-seven': smAndDown && isResolvingSeven }"
      >
        <div class="table-top">
          <div class="player-area opponent float-left" :class="{ 'active-turn': isActiveTurnSeat(leftSeat) }">
            <button
              type="button"
              class="player-header target-player-btn"
              :class="{
                'valid-target': isPlayerTarget(leftSeat),
              }"
              :disabled="!isPlayerTarget(leftSeat)"
              :data-cutthroat-player-target="leftSeat"
              @click="handlePlayerTargetClick(leftSeat)"
            >
              {{ seatLabel(leftSeat) }}
            </button>
            <div class="player-hand">
              <CutthroatCard
                v-for="card in leftHandCards"
                :key="card.key"
                :card="card.card"
                class="hand-card"
              />
            </div>
            <div class="player-stacks">
              <div class="stack-group">
                <div class="stack-title">
                  {{ t('cutthroat.game.points') }}
                </div>
                <div class="stack-list">
                  <div
                    v-for="stack in leftPointStacks"
                    :key="`left-point-${stack.baseToken}`"
                    class="stack"
                  >
                    <div class="stack-card-container">
                      <CutthroatCard
                        :card="stack.baseCard"
                        class="stack-base"
                        :clickable="isPointTarget(stack.baseToken)"
                        :is-valid-target="isPointTarget(stack.baseToken)"
                        :data-cutthroat-point-card="stack.baseToken"
                        @click="handlePointTargetClick(stack.baseToken)"
                      />
                      <div class="attachments-overlay">
                        <CutthroatCard
                          v-for="jack in stack.attachments"
                          :key="`left-jack-${stack.baseToken}-${jack.token}`"
                          :card="jack.card"
                          class="mini-card"
                          :is-jack="true"
                          :clickable="isJackTarget(jack.token)"
                          :is-valid-target="isJackTarget(jack.token)"
                          :data-cutthroat-jack-card="jack.token"
                          @click="handleJackTargetClick(jack.token)"
                        />
                      </div>
                    </div>
                    <div class="stack-meta">
                      <div class="stack-controller">
                        {{ seatLabel(stack.controller) }}
                      </div>
                    </div>
                  </div>
                  <div v-if="leftPointStacks.length === 0" class="stack-empty">
                    {{ t('cutthroat.game.noPoints') }}
                  </div>
                </div>
              </div>
              <div class="stack-group">
                <div class="stack-title">
                  {{ t('cutthroat.game.royals') }}
                </div>
                <div class="stack-list">
                  <div
                    v-for="stack in leftRoyalStacks"
                    :key="`left-royal-${stack.baseToken}`"
                    class="stack"
                  >
                    <div class="stack-card-container">
                      <CutthroatCard
                        :card="stack.baseCard"
                        class="stack-base"
                        :is-glasses="stack.isGlasses"
                        :clickable="isRoyalTarget(stack.baseToken)"
                        :is-valid-target="isRoyalTarget(stack.baseToken)"
                        :data-cutthroat-royal-card="stack.baseToken"
                        @click="handleRoyalTargetClick(stack.baseToken)"
                      />
                      <div class="attachments-overlay">
                        <CutthroatCard
                          v-for="joker in stack.attachments"
                          :key="`left-joker-${stack.baseToken}-${joker.token}`"
                          :card="joker.card"
                          class="mini-card"
                          :is-jack="true"
                          :clickable="isJokerTarget(joker.token)"
                          :is-valid-target="isJokerTarget(joker.token)"
                          :data-cutthroat-joker-card="joker.token"
                          @click="handleJokerTargetClick(joker.token)"
                        />
                      </div>
                    </div>
                    <div class="stack-meta">
                      <div class="stack-controller">
                        {{ seatLabel(stack.controller) }}
                      </div>
                    </div>
                  </div>
                  <div v-if="leftRoyalStacks.length === 0" class="stack-empty">
                    {{ t('cutthroat.game.noRoyals') }}
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="history-panel history-panel-desktop">
            <h3 class="history-title">
              {{ t('game.history.title') }}
            </h3>
            <div ref="logsContainerDesktop" class="history-logs">
              <p
                v-for="(log, index) in historyLines"
                :key="`cutthroat-log-desktop-${index}`"
                class="history-log"
                data-cy="cutthroat-history-log"
                data-cy-history-log="history-log"
              >
                {{ log }}
              </p>
              <p v-if="historyLines.length === 0" class="history-log history-log-empty">
                {{ t('cutthroat.game.noActions') }}
              </p>
            </div>
          </div>

          <div class="player-area opponent float-right" :class="{ 'active-turn': isActiveTurnSeat(rightSeat) }">
            <button
              type="button"
              class="player-header target-player-btn"
              :class="{
                'valid-target': isPlayerTarget(rightSeat),
              }"
              :disabled="!isPlayerTarget(rightSeat)"
              :data-cutthroat-player-target="rightSeat"
              @click="handlePlayerTargetClick(rightSeat)"
            >
              {{ seatLabel(rightSeat) }}
            </button>
            <div class="player-hand">
              <CutthroatCard
                v-for="card in rightHandCards"
                :key="card.key"
                :card="card.card"
                class="hand-card"
              />
            </div>
            <div class="player-stacks">
              <div class="stack-group">
                <div class="stack-title">
                  {{ t('cutthroat.game.points') }}
                </div>
                <div class="stack-list">
                  <div
                    v-for="stack in rightPointStacks"
                    :key="`right-point-${stack.baseToken}`"
                    class="stack"
                  >
                    <div class="stack-card-container">
                      <CutthroatCard
                        :card="stack.baseCard"
                        class="stack-base"
                        :clickable="isPointTarget(stack.baseToken)"
                        :is-valid-target="isPointTarget(stack.baseToken)"
                        :data-cutthroat-point-card="stack.baseToken"
                        @click="handlePointTargetClick(stack.baseToken)"
                      />
                      <div class="attachments-overlay">
                        <CutthroatCard
                          v-for="jack in stack.attachments"
                          :key="`right-jack-${stack.baseToken}-${jack.token}`"
                          :card="jack.card"
                          class="mini-card"
                          :is-jack="true"
                          :clickable="isJackTarget(jack.token)"
                          :is-valid-target="isJackTarget(jack.token)"
                          :data-cutthroat-jack-card="jack.token"
                          @click="handleJackTargetClick(jack.token)"
                        />
                      </div>
                    </div>
                    <div class="stack-meta">
                      <div class="stack-controller">
                        {{ seatLabel(stack.controller) }}
                      </div>
                    </div>
                  </div>
                  <div v-if="rightPointStacks.length === 0" class="stack-empty">
                    {{ t('cutthroat.game.noPoints') }}
                  </div>
                </div>
              </div>
              <div class="stack-group">
                <div class="stack-title">
                  {{ t('cutthroat.game.royals') }}
                </div>
                <div class="stack-list">
                  <div
                    v-for="stack in rightRoyalStacks"
                    :key="`right-royal-${stack.baseToken}`"
                    class="stack"
                  >
                    <div class="stack-card-container">
                      <CutthroatCard
                        :card="stack.baseCard"
                        class="stack-base"
                        :is-glasses="stack.isGlasses"
                        :clickable="isRoyalTarget(stack.baseToken)"
                        :is-valid-target="isRoyalTarget(stack.baseToken)"
                        :data-cutthroat-royal-card="stack.baseToken"
                        @click="handleRoyalTargetClick(stack.baseToken)"
                      />
                      <div class="attachments-overlay">
                        <CutthroatCard
                          v-for="joker in stack.attachments"
                          :key="`right-joker-${stack.baseToken}-${joker.token}`"
                          :card="joker.card"
                          class="mini-card"
                          :is-jack="true"
                          :clickable="isJokerTarget(joker.token)"
                          :is-valid-target="isJokerTarget(joker.token)"
                          :data-cutthroat-joker-card="joker.token"
                          @click="handleJokerTargetClick(joker.token)"
                        />
                      </div>
                    </div>
                    <div class="stack-meta">
                      <div class="stack-controller">
                        {{ seatLabel(stack.controller) }}
                      </div>
                    </div>
                  </div>
                  <div v-if="rightRoyalStacks.length === 0" class="stack-empty">
                    {{ t('cutthroat.game.noRoyals') }}
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="table-center">
          <div
            class="pile"
            :class="{ clickable: canUseDeck }"
            data-cy="cutthroat-deck"
            @click="handleDeckClick"
          >
            <div class="pile-title">
              {{ t('cutthroat.game.deck') }} ({{ playerView.deck_count }})
            </div>
            <div class="pile-cards">
              <div
                v-if="isResolvingSeven"
                class="reveal-group"
                @click.stop
              >
                <button
                  v-for="reveal in revealedCardEntries"
                  :key="`reveal-${reveal.index}-${reveal.token}`"
                  class="reveal-card"
                  :class="{
                    selected: isRevealSelected(reveal.index),
                    clickable: isRevealSelectable(reveal.index),
                  }"
                  :disabled="!isRevealSelectable(reveal.index)"
                  :data-cy="`cutthroat-reveal-${reveal.index}`"
                  @click="handleRevealClick(reveal.index)"
                >
                  <CutthroatCard
                    :card="reveal.card"
                    class="hand-card"
                    :is-selected="isRevealSelected(reveal.index)"
                    :clickable="isRevealSelectable(reveal.index)"
                  />
                </button>
              </div>
              <div v-else class="deck-face" />
            </div>
          </div>
          <CutthroatScrapPile
            :scrap-tokens="playerView.scrap"
            :is-resolving-three-turn="isResolvingThreeTurn"
            :is-action-disabled="isActionDisabled"
            :is-straightened="store.isScrapStraightened"
            @pick-scrap-card="handleScrapCardClick"
            @request-scrap-straighten="handleRequestScrapStraighten"
          />
        </div>

        <div class="table-bottom">
          <div class="player-area me">
            <div class="player-header">
              <span>{{ seatLabel(mySeat) }}</span>
              <span
                class="turn-status"
                :class="{ 'my-turn': isMyTurn, 'text-black': isMyTurn, 'text-white': !isMyTurn }"
              >
                {{ turnLabel }}
              </span>
            </div>
            <div class="player-stacks">
              <div class="stack-group">
                <div class="stack-title">
                  {{ t('cutthroat.game.points') }}
                </div>
                <div class="stack-list">
                  <div
                    v-for="stack in myPointStacks"
                    :key="`me-point-${stack.baseToken}`"
                    class="stack"
                  >
                    <div class="stack-card-container">
                      <CutthroatCard
                        :card="stack.baseCard"
                        class="stack-base"
                        :clickable="isPointTarget(stack.baseToken)"
                        :is-valid-target="isPointTarget(stack.baseToken)"
                        :data-cutthroat-point-card="stack.baseToken"
                        @click="handlePointTargetClick(stack.baseToken)"
                      />
                      <div class="attachments-overlay">
                        <CutthroatCard
                          v-for="jack in stack.attachments"
                          :key="`me-jack-${stack.baseToken}-${jack.token}`"
                          :card="jack.card"
                          class="mini-card"
                          :is-jack="true"
                          :clickable="isJackTarget(jack.token)"
                          :is-valid-target="isJackTarget(jack.token)"
                          :data-cutthroat-jack-card="jack.token"
                          @click="handleJackTargetClick(jack.token)"
                        />
                      </div>
                    </div>
                    <div class="stack-meta">
                      <div class="stack-controller">
                        {{ seatLabel(stack.controller) }}
                      </div>
                    </div>
                  </div>
                  <div v-if="myPointStacks.length === 0" class="stack-empty">
                    {{ t('cutthroat.game.noPoints') }}
                  </div>
                </div>
              </div>
              <div class="stack-group">
                <div class="stack-title">
                  {{ t('cutthroat.game.royals') }}
                </div>
                <div class="stack-list">
                  <div
                    v-for="stack in myRoyalStacks"
                    :key="`me-royal-${stack.baseToken}`"
                    class="stack"
                  >
                    <div class="stack-card-container">
                      <CutthroatCard
                        :card="stack.baseCard"
                        class="stack-base"
                        :is-glasses="stack.isGlasses"
                        :clickable="isRoyalTarget(stack.baseToken)"
                        :is-valid-target="isRoyalTarget(stack.baseToken)"
                        :data-cutthroat-royal-card="stack.baseToken"
                        @click="handleRoyalTargetClick(stack.baseToken)"
                      />
                      <div class="attachments-overlay">
                        <CutthroatCard
                          v-for="joker in stack.attachments"
                          :key="`me-joker-${stack.baseToken}-${joker.token}`"
                          :card="joker.card"
                          class="mini-card"
                          :is-jack="true"
                          :clickable="isJokerTarget(joker.token)"
                          :is-valid-target="isJokerTarget(joker.token)"
                          :data-cutthroat-joker-card="joker.token"
                          @click="handleJokerTargetClick(joker.token)"
                        />
                      </div>
                    </div>
                    <div class="stack-meta">
                      <div class="stack-controller">
                        {{ seatLabel(stack.controller) }}
                      </div>
                    </div>
                  </div>
                  <div v-if="myRoyalStacks.length === 0" class="stack-empty">
                    {{ t('cutthroat.game.noRoyals') }}
                  </div>
                </div>
              </div>
            </div>
            <div class="frozen-zone">
              <div class="stack-title">
                {{ t('cutthroat.game.frozen') }}
              </div>
              <div
                v-if="myFrozenCards.length === 0"
                class="stack-empty"
              >
                {{ t('cutthroat.game.noFrozen') }}
              </div>
              <div v-else class="player-hand">
                <CutthroatCard
                  v-for="card in myFrozenCards"
                  :key="`me-frozen-${card.key}`"
                  :card="card.card"
                  class="hand-card"
                  :is-frozen="true"
                />
              </div>
            </div>
            <div
              v-if="isTargeting && selectedSourceCard && !showFourPlayerTargetDialog"
              class="player-hand-targeting-overlay"
            >
              <CutthroatTargetSelectionOverlay
                :selected-card="selectedSourceCard"
                :move-display-name="formatChoiceType(selectedChoice)"
                @cancel="cancelTargeting"
              />
            </div>
            <div v-else class="player-hand me" :class="{ 'my-turn': isMyTurn }">
              <CutthroatCard
                v-for="card in myHandCards"
                :key="card.key"
                :card="card.card"
                class="hand-card"
                :clickable="isHandSourceSelectable(card)"
                :is-selected="isHandSourceSelected(card)"
                :is-frozen="isFrozenToken(card.token)"
                :data-cutthroat-hand-card="card.token"
                @click="handleHandCardClick(card)"
              />
            </div>
          </div>
        </div>

        <details v-if="isDevMode" class="debug-actions">
          <summary>{{ t('cutthroat.game.debugActions') }}</summary>
          <div class="debug-actions-grid">
            <v-btn
              v-for="(action, index) in legalActions"
              :key="`debug-action-${index}`"
              color="surface-1"
              variant="outlined"
              size="small"
              :disabled="isActionDisabled"
              :loading="isActionLoading(action)"
              @click="sendResolvedAction(action)"
            >
              {{ formatAction(action) }}
            </v-btn>
          </div>
        </details>
      </div>
    </template>

    <CutthroatMoveChoiceOverlay
      :model-value="showMoveChoiceOverlay"
      :selected-card="selectedSourceCard"
      :is-frozen="selectedSourceIsFrozen"
      :move-choices="moveChoiceCards"
      :disabled="isActionDisabled"
      @cancel="clearInteractionState"
      @choose-move="chooseMove"
    />

    <BaseDialog
      id="cutthroat-four-player-target-dialog"
      :model-value="showFourPlayerTargetDialog"
      :title="t('cutthroat.game.chooseTargetTitle', { action: formatChoiceType(selectedChoice) })"
      :persistent="true"
      :max-width="520"
    >
      <template #body>
        <p class="mb-4">
          {{ t('cutthroat.game.targetPlayer') }}
        </p>
      </template>
      <template #actions>
        <div class="d-flex flex-wrap justify-center ga-2 w-100">
          <v-btn
            v-for="target in playerTargetChoices"
            :key="`four-target-seat-${target.seat}`"
            color="primary"
            variant="flat"
            :data-cy="`cutthroat-four-target-player-${target.seat}`"
            @click="handlePlayerTargetClick(target.seat)"
          >
            {{ seatLabel(target.seat) }}
          </v-btn>
          <v-btn
            color="surface-1"
            variant="outlined"
            data-cy="cutthroat-four-target-cancel"
            @click="cancelTargeting"
          >
            {{ t('cutthroat.game.cancel') }}
          </v-btn>
        </div>
      </template>
    </BaseDialog>

    <CounterDialog
      :model-value="showCounterDialog"
      :one-off="counterDialogOneOff"
      :target="counterDialogTarget"
      :twos-in-hand="counterDialogTwosInHand"
      :twos-played="counterDialogTwosPlayed"
      @counter="handleCounterTwoFromDialog"
      @resolve="handleCounterPass"
    />

    <CannotCounterDialog
      :model-value="showCannotCounterDialog"
      :one-off="counterDialogOneOff"
      :target="counterDialogTarget"
      :opponent-queen-count="0"
      :player-two-count="counterDialogTwosInHand.length"
      :twos-played="counterDialogTwosPlayed"
      @resolve="handleCounterPass"
    />

    <div
      v-if="isDevMode && counterDialogInvariantError"
      class="counter-context-error"
      data-cy="cutthroat-counter-context-error"
    >
      Counter context unavailable from tokenlog while counter actions are legal.
    </div>

    <BaseDialog
      id="cutthroat-four-discard-dialog"
      :model-value="showResolveFourDialog"
      :title="t('game.dialogs.four.discardTwoCards')"
      minimizable
    >
      <template #body>
        <p class="mb-4">
          {{ t('game.dialogs.four.opponentHasResolved') }}
        </p>
        <div class="d-flex flex-wrap card-container">
          <CutthroatCard
            v-for="card in resolveFourHandCards"
            :key="`resolve-four-${card.token}`"
            :card="card.card"
            :clickable="true"
            :is-selected="selectedResolveFourTokens.includes(card.token)"
            :data-discard-card="formatCardToken(card.token)"
            @click="toggleResolveFourCard(card.token)"
          />
        </div>
      </template>
      <template #actions>
        <v-btn
          color="surface-1"
          variant="flat"
          data-cy="submit-four-dialog"
          :disabled="!canSubmitResolveFour"
          @click="submitResolveFourDiscard"
        >
          {{ t('game.dialogs.four.discard') }}
        </v-btn>
      </template>
    </BaseDialog>

    <BaseDialog
      id="cutthroat-five-discard-dialog"
      :model-value="showResolveFiveDialog"
      :title="resolveFiveDialogTitle"
      minimizable
    >
      <template #body>
        <p class="mb-4">
          {{ resolveFiveDialogBody }}
        </p>
        <div v-if="resolveFiveHandCards.length > 0" class="d-flex flex-wrap card-container justify-center">
          <CutthroatCard
            v-for="card in resolveFiveHandCards"
            :key="`resolve-five-${card.token}`"
            :card="card.card"
            :clickable="true"
            :is-selected="selectedResolveFiveToken === card.token"
            :data-discard-card="formatCardToken(card.token)"
            @click="selectedResolveFiveToken = card.token"
          />
        </div>
      </template>
      <template #actions>
        <v-btn
          color="surface-1"
          variant="flat"
          data-cy="submit-five-dialog"
          :disabled="!canSubmitResolveFive"
          @click="submitResolveFiveDiscard"
        >
          {{ resolveFiveDialogButton }}
        </v-btn>
      </template>
    </BaseDialog>

    <BaseDialog
      id="cutthroat-game-over-dialog"
      :model-value="isFinished"
      :title="t('cutthroat.game.gameOverTitle')"
      :persistent="true"
      :max-width="560"
    >
      <template #body>
        <div class="finished-subtitle">
          {{ gameResultText }}
        </div>
      </template>
      <template #actions>
        <div class="finished-actions">
          <v-btn
            size="small"
            color="primary"
            variant="flat"
            :loading="rematchLoading"
            :disabled="rematchLoading"
            data-cy="cutthroat-rematch-btn"
            @click="handleRematch"
          >
            {{ t('game.dialogs.gameOverDialog.rematch') }}
          </v-btn>
          <v-btn
            size="small"
            color="surface-1"
            variant="outlined"
            data-cy="cutthroat-back-home-btn"
            @click="goToHome"
          >
            {{ t('game.dialogs.gameOverDialog.goHome') }}
          </v-btn>
        </div>
      </template>
    </BaseDialog>
  </div>
</template>

<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { useDisplay } from 'vuetify';
import BaseDialog from '@/components/BaseDialog.vue';
import { useCutthroatStore } from '@/stores/cutthroat';
import { useSnackbarStore } from '@/stores/snackbar';
import CounterDialog from '@/routes/game/components/dialogs/components/CounterDialog.vue';
import CannotCounterDialog from '@/routes/game/components/dialogs/components/CannotCounterDialog.vue';
import CutthroatCard from '@/routes/cutthroat/components/CutthroatCard.vue';
import CutthroatScrapPile from '@/routes/cutthroat/components/CutthroatScrapPile.vue';
import CutthroatMoveChoiceOverlay from '@/routes/cutthroat/components/CutthroatMoveChoiceOverlay.vue';
import CutthroatTargetSelectionOverlay from '@/routes/cutthroat/components/CutthroatTargetSelectionOverlay.vue';
import { parseCardToken, publicCardToDisplay, formatCardToken } from '@/util/cutthroat-cards';
import {
  deriveCounterDialogContextFromTokenlog,
  deriveMoveChoicesForSource,
  deriveTargetsForChoice,
  deriveCutthroatDialogState,
  extractActionSource,
  findMatchingAction,
  getCutthroatGameResult,
  isActionInteractionDisabled,
  isCutthroatGameFinished,
  makeSeatLabel,
} from '@/routes/cutthroat/cutthroat-view-helpers';

const route = useRoute();
const router = useRouter();
const { t } = useI18n();
const { smAndDown } = useDisplay();
const store = useCutthroatStore();
const snackbarStore = useSnackbarStore();

const isDevMode = import.meta.env.DEV;

const gameId = computed(() => Number(route.params.gameId));
const playerView = computed(() => store.playerView);
const phase = computed(() => playerView.value?.phase ?? null);
const phaseType = computed(() => phase.value?.type ?? null);
const phaseData = computed(() => phase.value?.data ?? {});
const legalActions = computed(() => store.legalActions ?? []);
const historyLines = computed(() => store.logTail ?? []);
const seatEntries = computed(() => store.lobby?.seats ?? []);

const mySeat = computed(() => store.seat ?? 0);
const leftSeat = computed(() => (mySeat.value + 1) % 3);
const rightSeat = computed(() => (mySeat.value + 2) % 3);
const activeTurnSeat = computed(() => playerView.value?.turn ?? null);
const isMyTurn = computed(() => !isFinished.value && activeTurnSeat.value === mySeat.value);

const actionInFlight = ref(false);
const actionInFlightKey = ref('');
const selectedSource = ref(null);
const selectedChoice = ref(null);
const selectedResolveFourTokens = ref([]);
const selectedResolveFiveToken = ref(null);
const rematchLoading = ref(false);
const showHistoryDrawer = ref(false);
const logsContainerDesktop = ref(null);
const logsContainerDrawer = ref(null);

function setBrowserHeightVariable() {
  const viewportHeight = window.innerHeight * 0.01;
  document.documentElement.style.setProperty('--browserHeight', `${viewportHeight}px`);
}

const isMainPhase = computed(() => phaseType.value === 'Main');
const isCounteringPhase = computed(() => phaseType.value === 'Countering');
const isResolvingThree = computed(() => phaseType.value === 'ResolvingThree');
const isResolvingFour = computed(() => phaseType.value === 'ResolvingFour');
const isResolvingFive = computed(() => phaseType.value === 'ResolvingFive');
const isResolvingSeven = computed(() => phaseType.value === 'ResolvingSeven');
const isFinished = computed(() => isCutthroatGameFinished(store.status));
const isActionDisabled = computed(() => isActionInteractionDisabled(store.status, actionInFlight.value));

const isCounterTurn = computed(() => {
  if (!isCounteringPhase.value) {return false;}
  return hasCounterPassAction.value;
});

const isResolvingThreeTurn = computed(() => {
  if (!isResolvingThree.value) {return false;}
  return legalActions.value.some((action) => action?.type === 'ResolveThreePick');
});

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

const selectedSourceChoices = computed(() => {
  return deriveMoveChoicesForSource(legalActions.value, selectedSource.value);
});

const selectedChoiceTargets = computed(() => {
  if (!selectedSource.value || !selectedChoice.value) {return [];}
  return deriveTargetsForChoice(legalActions.value, selectedSource.value, selectedChoice.value);
});

const targetKeySet = computed(() => {
  return new Set(selectedChoiceTargets.value.map((target) => target.key));
});

const dialogState = computed(() => {
  return deriveCutthroatDialogState({
    phaseType: phaseType.value,
    legalActions: legalActions.value,
    selectedSource: selectedSource.value,
    selectedChoice: selectedChoice.value,
    targets: selectedChoiceTargets.value,
  });
});

const isTargeting = computed(() => {
  return !!selectedSource.value && !!selectedChoice.value && selectedChoiceTargets.value.length > 0;
});

const playerTargetChoices = computed(() => {
  return dialogState.value.playerTargetSeats.map((seat) => ({
    targetType: 'player',
    seat,
    key: `player:${seat}`,
  }));
});

const showFourPlayerTargetDialog = computed(() => {
  return isTargeting.value && dialogState.value.showFourPlayerTargetDialog;
});

const showMoveChoiceOverlay = computed(() => {
  return !!selectedSource.value
    && !selectedChoice.value
    && selectedSourceChoices.value.length > 0;
});

const hasCounterPassAction = computed(() => {
  return dialogState.value.hasCounterPass;
});

const localHandActionTokens = computed(() => {
  const deduped = [];
  const seen = new Set();
  legalActions.value.forEach((action) => {
    const source = extractActionSource(action);
    if (!source || source.zone !== 'hand' || !source.token) {return;}
    if (seen.has(source.token)) {return;}
    seen.add(source.token);
    deduped.push(source.token);
  });
  return deduped;
});

const counterTwoOptions = computed(() => {
  return dialogState.value.counterTwoTokens;
});

const counterContext = computed(() => {
  if (!isCounteringPhase.value) {return null;}
  return deriveCounterDialogContextFromTokenlog(store.tokenlog);
});

const counterDialogOneOff = computed(() => {
  return cardTokenToDialogCard(counterContext.value?.oneOffCardToken ?? null);
});

const counterDialogTarget = computed(() => {
  const target = counterContext.value?.oneOffTarget ?? null;
  if (!target) {return null;}
  switch (target.type) {
    case 'Point':
    case 'Royal':
    case 'Jack':
    case 'Joker':
      return cardTokenToDialogCard(target.token);
    default:
      return null;
  }
});

const counterDialogTwosPlayed = computed(() => {
  return (counterContext.value?.twosPlayed ?? [])
    .map((token) => {
      const card = cardTokenToDialogCard(token);
      if (!card) {return null;}
      return {
        ...card,
        id: token,
      };
    })
    .filter(Boolean);
});

const counterDialogTwosInHand = computed(() => {
  return counterTwoOptions.value
    .map((token) => cardTokenToDialogCard(token))
    .filter(Boolean);
});

const showCounterDialog = computed(() => {
  return dialogState.value.showCounterDialog;
});

const showCannotCounterDialog = computed(() => {
  return dialogState.value.showCannotCounterDialog;
});

const counterDialogInvariantError = computed(() => {
  if (!isCounteringPhase.value) {return false;}
  if (!showCounterDialog.value && !showCannotCounterDialog.value) {return false;}
  return !counterContext.value || !counterDialogOneOff.value;
});

const canUseDeck = computed(() => {
  if (isActionDisabled.value || isFinished.value || !isMainPhase.value) {return false;}
  const draw = findMatchingAction(legalActions.value, { zone: 'deck' }, 'draw');
  const pass = findMatchingAction(legalActions.value, { zone: 'deck' }, 'pass');
  return !!(draw || pass);
});

const selectedSourceCard = computed(() => {
  if (!selectedSource.value) {return null;}
  if (selectedSource.value.zone === 'hand') {
    const found = myHandCards.value.find((card) => card.token === selectedSource.value.token);
    return found?.card ?? parseCardToken(selectedSource.value.token);
  }
  if (selectedSource.value.zone === 'reveal') {
    const found = revealedCardEntries.value.find((entry) => entry.index === selectedSource.value.index);
    return found?.card ?? null;
  }
  if (selectedSource.value.zone === 'scrap') {
    return parseCardToken(selectedSource.value.token);
  }
  return null;
});

const selectedSourceIsFrozen = computed(() => {
  if (!selectedSource.value || selectedSource.value.zone !== 'hand') {return false;}
  return isFrozenToken(selectedSource.value.token);
});

const moveChoiceCards = computed(() => {
  return selectedSourceChoices.value.map((choice) => ({
    type: choice.type,
    displayName: formatChoiceType(choice.type),
    moveDescription: describeChoice(choice.type),
  }));
});

const gameResultText = computed(() => {
  if (!isFinished.value) {return '';}
  const result = getCutthroatGameResult(store.status, playerView.value);
  if (result.type === 'winner' && result.seat !== null && result.seat !== undefined) {
    return t('cutthroat.game.gameOverWinner', {
      player: seatLabel(result.seat),
    });
  }
  if (result.type === 'draw') {
    return t('cutthroat.game.gameOverDraw');
  }
  return t('cutthroat.game.gameOverGeneric');
});

const turnLabel = computed(() => {
  if (!playerView.value) {return '';}
  if (isFinished.value) {return t('cutthroat.game.gameOverTitle');}
  return playerView.value.turn === mySeat.value ? t('game.turn.yourTurn') : t('game.turn.opponentTurn');
});

function isActiveTurnSeat(seat) {
  if (isFinished.value) {return false;}
  return activeTurnSeat.value === seat;
}

const resolveFiveActions = computed(() => {
  return legalActions.value.filter((action) => action?.type === 'ResolveFiveDiscard');
});

const resolveFourDiscardTokens = computed(() => {
  return dialogState.value.resolveFourTokens;
});

const resolveFiveDiscardTokens = computed(() => {
  return dialogState.value.resolveFiveTokens;
});

const showResolveFourDialog = computed(() => {
  return dialogState.value.showResolveFourDialog;
});

const showResolveFiveDialog = computed(() => {
  return dialogState.value.showResolveFiveDialog;
});

const resolveFourHandCards = computed(() => {
  return resolveFourDiscardTokens.value.map((token, index) => ({
    token,
    key: `resolve-four-${token}-${index}`,
    card: parseCardToken(token),
  }));
});

const resolveFiveHandCards = computed(() => {
  return resolveFiveDiscardTokens.value.map((token, index) => ({
    token,
    key: `resolve-five-${token}-${index}`,
    card: parseCardToken(token),
  }));
});

const canSubmitResolveFour = computed(() => {
  if (resolveFourHandCards.value.length === 0) {return false;}
  const maxSelectable = Math.min(2, resolveFourHandCards.value.length);
  return selectedResolveFourTokens.value.length === maxSelectable;
});

const canSubmitResolveFive = computed(() => {
  if (resolveFiveHandCards.value.length === 0) {return true;}
  return !!selectedResolveFiveToken.value;
});

const resolveFiveDialogTitle = computed(() => {
  return t(resolveFiveHandCards.value.length > 0 ? 'game.dialogs.five.discardAndDraw' : 'game.dialogs.five.nice');
});

const resolveFiveDialogBody = computed(() => {
  return t(resolveFiveHandCards.value.length > 0 ? 'game.dialogs.five.resolveFive' : 'game.dialogs.five.resolveFiveNoCards');
});

const resolveFiveDialogButton = computed(() => {
  return t(resolveFiveHandCards.value.length > 0 ? 'game.dialogs.five.discardAndDraw' : 'rules.draw');
});

function playerForSeat(seat) {
  return playerView.value?.players?.find((player) => player.seat === seat);
}

function handFor(seat) {
  const player = playerForSeat(seat);
  if (!player) {return [];}
  const entries = player.hand.map((card, index) => {
    const token = card?.type === 'Known' ? card.data : null;
    return {
      token,
      key: token ?? `hidden-${seat}-${index}`,
      card: publicCardToDisplay(card),
      isKnown: card?.type === 'Known',
    };
  });

  if (seat !== mySeat.value) {
    return entries;
  }

  const knownTokens = new Set(entries.filter((entry) => entry.isKnown && entry.token).map((entry) => entry.token));
  const extraTokens = localHandActionTokens.value.filter((token) => !knownTokens.has(token));
  if (extraTokens.length === 0) {
    return entries;
  }

  const hydratedEntries = entries.map((entry, index) => {
    if (entry.isKnown || extraTokens.length === 0) {return entry;}
    const inferredToken = extraTokens.shift();
    if (!inferredToken) {return entry;}
    return {
      token: inferredToken,
      key: `inferred-${inferredToken}-${index}`,
      card: parseCardToken(inferredToken),
      isKnown: true,
    };
  });

  extraTokens.forEach((token, index) => {
    hydratedEntries.push({
      token,
      key: `inferred-extra-${token}-${index}`,
      card: parseCardToken(token),
      isKnown: true,
    });
  });

  return hydratedEntries;
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

function actionKey(action) {
  return JSON.stringify(action ?? {});
}

function cardNameForRankSuit(rank, suit) {
  const rankText = {
    1: 'A',
    11: 'J',
    12: 'Q',
    13: 'K',
  }[rank] ?? String(rank);
  const suitText = [ '♣️', '♦️', '♥️', '♠️' ][suit] ?? '';
  return `${rankText}${suitText}`;
}

function cardTokenToDialogCard(token) {
  if (!token) {return null;}
  const card = parseCardToken(token);
  if (!card || card.kind !== 'standard') {
    const normalized = String(token)
      .trim()
      .toUpperCase();
    const match = normalized.match(/^(10|[2-9AJQKT])([CDHS])$/);
    if (!match) {return null;}
    const [ , rankText, suitText ] = match;
    const rank = {
      A: 1,
      T: 10,
      J: 11,
      Q: 12,
      K: 13,
    }[rankText] ?? Number(rankText);
    const suit = {
      C: 0,
      D: 1,
      H: 2,
      S: 3,
    }[suitText];
    if (!Number.isFinite(rank) || suit === undefined) {return null;}
    return {
      id: token,
      rank,
      suit,
      name: cardNameForRankSuit(rank, suit),
    };
  }
  return {
    id: token,
    rank: card.rank,
    suit: card.suit,
    name: cardNameForRankSuit(card.rank, card.suit),
  };
}

function sameSource(a, b) {
  if (!a || !b) {return false;}
  return a.zone === b.zone
    && a.token === b.token
    && a.index === b.index;
}

function makeTargetKey(target) {
  if (!target || !target.targetType) {return '';}
  if (target.targetType === 'player') {
    return `player:${target.seat}`;
  }
  if (target.token !== undefined && target.token !== null) {
    return `${target.targetType}:${target.token}`;
  }
  return target.targetType;
}

function isActionLoading(action) {
  if (!actionInFlight.value) {return false;}
  return actionInFlightKey.value === actionKey(action);
}

function clearInteractionState() {
  selectedSource.value = null;
  selectedChoice.value = null;
}

function cancelTargeting() {
  selectedChoice.value = null;
}

function syncInteractionState() {
  if (!selectedSource.value) {return;}

  const choices = deriveMoveChoicesForSource(legalActions.value, selectedSource.value);
  if (choices.length === 0) {
    clearInteractionState();
    return;
  }

  if (!selectedChoice.value) {return;}

  const matchingChoice = choices.some((choice) => choice.type === selectedChoice.value);
  if (!matchingChoice) {
    selectedChoice.value = null;
    return;
  }

  const targets = deriveTargetsForChoice(legalActions.value, selectedSource.value, selectedChoice.value);
  if (targets.length === 0) {
    selectedChoice.value = null;
  }
}

function isFrozenToken(token) {
  if (!token) {return false;}
  return myFrozenTokens.value.has(token);
}

function isHandSourceSelectable(handCard) {
  if (!handCard?.isKnown || !handCard?.token || isActionDisabled.value || isFinished.value) {return false;}
  const source = {
    zone: 'hand',
    token: handCard.token,
  };
  return deriveMoveChoicesForSource(legalActions.value, source).length > 0;
}

function isHandSourceSelected(handCard) {
  if (!handCard?.token || !selectedSource.value) {return false;}
  return selectedSource.value.zone === 'hand' && selectedSource.value.token === handCard.token;
}

function isRevealSelectable(index) {
  const source = {
    zone: 'reveal',
    index,
  };
  return deriveMoveChoicesForSource(legalActions.value, source).length > 0;
}

function isRevealSelected(index) {
  if (!selectedSource.value) {return false;}
  return selectedSource.value.zone === 'reveal' && selectedSource.value.index === index;
}

function hasTarget(target) {
  return targetKeySet.value.has(makeTargetKey(target));
}

function isPointTarget(token) {
  return isTargeting.value && hasTarget({
    targetType: 'point',
    token,
  });
}

function isRoyalTarget(token) {
  return isTargeting.value && hasTarget({
    targetType: 'royal',
    token,
  });
}

function isJackTarget(token) {
  return isTargeting.value && hasTarget({
    targetType: 'jack',
    token,
  });
}

function isJokerTarget(token) {
  return isTargeting.value && hasTarget({
    targetType: 'joker',
    token,
  });
}

function isPlayerTarget(seat) {
  return isTargeting.value && hasTarget({
    targetType: 'player',
    seat,
  });
}

async function sendResolvedAction(action) {
  if (!action || isActionDisabled.value) {return false;}

  actionInFlight.value = true;
  actionInFlightKey.value = actionKey(action);
  let succeeded = false;

  try {
    await store.sendAction(action);
    succeeded = true;
  } catch (err) {
    if (!store.lastError) {
      snackbarStore.alert(err?.message ?? t('cutthroat.game.actionFailed'));
    }
  } finally {
    actionInFlight.value = false;
    actionInFlightKey.value = '';
  }

  if (succeeded) {
    clearInteractionState();
  } else {
    syncInteractionState();
  }

  return succeeded;
}

async function executeSourceChoice(source, choiceType, target = null) {
  const action = findMatchingAction(legalActions.value, source, choiceType, target);
  if (!action) {return;}
  await sendResolvedAction(action);
}

function chooseMove(choiceType) {
  if (!selectedSource.value || isActionDisabled.value) {return;}

  const targets = deriveTargetsForChoice(legalActions.value, selectedSource.value, choiceType);
  if (targets.length === 0) {
    executeSourceChoice(selectedSource.value, choiceType);
    return;
  }

  selectedChoice.value = choiceType;
}

function resolveTargetSelection(target) {
  if (!selectedSource.value || !selectedChoice.value || !isTargeting.value) {return;}
  if (!hasTarget(target)) {return;}
  executeSourceChoice(selectedSource.value, selectedChoice.value, target);
}

async function handleDeckClick() {
  if (!canUseDeck.value) {return;}
  const draw = findMatchingAction(legalActions.value, { zone: 'deck' }, 'draw');
  const pass = findMatchingAction(legalActions.value, { zone: 'deck' }, 'pass');
  await sendResolvedAction(draw ?? pass);
}

async function handleCounterPass() {
  const action = findMatchingAction(legalActions.value, { zone: 'counter', token: 'pass' }, 'counterPass');
  await sendResolvedAction(action);
}

async function handleCounterTwo(twoToken) {
  if (!twoToken) {return;}
  const action = findMatchingAction(
    legalActions.value,
    { zone: 'hand', token: twoToken },
    'counterTwo',
  );
  await sendResolvedAction(action);
}

async function handleCounterTwoFromDialog(twoId) {
  if (!twoId) {return;}
  await handleCounterTwo(twoId);
}

function toggleResolveFourCard(token) {
  if (!token) {return;}
  const selected = selectedResolveFourTokens.value;
  if (selected.includes(token)) {
    selectedResolveFourTokens.value = selected.filter((entry) => entry !== token);
    return;
  }
  const maxSelectable = Math.min(2, resolveFourHandCards.value.length);
  const next = [ ...selected, token ];
  if (next.length > maxSelectable) {
    next.shift();
  }
  selectedResolveFourTokens.value = next;
}

async function submitResolveFourDiscard() {
  if (!canSubmitResolveFour.value) {return;}
  const tokens = [ ...selectedResolveFourTokens.value ];
  for (const token of tokens) {
    const action = findMatchingAction(
      legalActions.value,
      { zone: 'hand', token },
      'resolveFourDiscard',
    );
    if (!action) {continue;}
    // Sequentially send actions so each discard respects server-side state/version updates.
    await sendResolvedAction(action);
  }
  selectedResolveFourTokens.value = [];
}

async function submitResolveFiveDiscard() {
  if (!canSubmitResolveFive.value) {return;}
  let action = null;
  if (selectedResolveFiveToken.value) {
    action = findMatchingAction(
      legalActions.value,
      { zone: 'hand', token: selectedResolveFiveToken.value },
      'resolveFiveDiscard',
    );
  } else {
    action = resolveFiveActions.value[0] ?? null;
  }
  await sendResolvedAction(action);
  selectedResolveFiveToken.value = null;
}

async function handleHandCardClick(handCard) {
  if (!isHandSourceSelectable(handCard)) {return;}

  const source = {
    zone: 'hand',
    token: handCard.token,
  };
  const choices = deriveMoveChoicesForSource(legalActions.value, source);

  if (
    isResolvingFour.value
    || isResolvingFive.value
    || (isCounterTurn.value && choices.length === 1 && choices[0].type === 'counterTwo')
  ) {
    await executeSourceChoice(source, choices[0].type);
    return;
  }

  if (sameSource(selectedSource.value, source) && !selectedChoice.value) {
    clearInteractionState();
    return;
  }

  selectedSource.value = source;
  selectedChoice.value = null;
}

function handleRevealClick(index) {
  if (isActionDisabled.value) {return;}

  const source = {
    zone: 'reveal',
    index,
  };

  if (deriveMoveChoicesForSource(legalActions.value, source).length === 0) {return;}

  if (sameSource(selectedSource.value, source) && !selectedChoice.value) {
    clearInteractionState();
    return;
  }

  selectedSource.value = source;
  selectedChoice.value = null;
}

async function handleScrapCardClick(token) {
  if (!isResolvingThreeTurn.value || isActionDisabled.value) {return;}

  await executeSourceChoice(
    {
      zone: 'scrap',
      token,
    },
    'resolveThreePick',
  );
}

function handleRequestScrapStraighten() {
  try {
    store.sendScrapStraighten();
  } catch (err) {
    snackbarStore.alert(err?.message ?? t('cutthroat.game.actionFailed'));
  }
}

function handlePointTargetClick(token) {
  resolveTargetSelection({
    targetType: 'point',
    token,
  });
}

function handleRoyalTargetClick(token) {
  resolveTargetSelection({
    targetType: 'royal',
    token,
  });
}

function handleJackTargetClick(token) {
  resolveTargetSelection({
    targetType: 'jack',
    token,
  });
}

function handleJokerTargetClick(token) {
  resolveTargetSelection({
    targetType: 'joker',
    token,
  });
}

function handlePlayerTargetClick(seat) {
  resolveTargetSelection({
    targetType: 'player',
    seat,
  });
}

function formatChoiceType(choiceType) {
  switch (choiceType) {
    case 'draw':
      return t('cutthroat.game.draw');
    case 'pass':
      return t('cutthroat.game.pass');
    case 'points':
      return t('game.moves.points.displayName');
    case 'scuttle':
      return t('cutthroat.game.scuttle');
    case 'royal':
      return t('game.moves.royal.displayName');
    case 'jack':
      return 'Jack';
    case 'joker':
      return t('cutthroat.game.playJoker');
    case 'oneOff':
      return 'One-Off';
    case 'discard':
      return t('cutthroat.game.discard');
    case 'counterTwo':
      return t('cutthroat.game.counterTwo');
    case 'counterPass':
      return t('cutthroat.game.counterPassAction');
    case 'resolveThreePick':
      return t('cutthroat.game.pickFromScrap');
    case 'resolveFourDiscard':
    case 'resolveFiveDiscard':
      return t('cutthroat.game.discard');
    default:
      return choiceType ?? t('cutthroat.game.action');
  }
}

function describeChoice(choiceType) {
  const card = selectedSourceCard.value;
  const rank = card?.rank;
  switch (choiceType) {
    case 'draw':
      return t('cutthroat.game.draw');
    case 'pass':
      return t('cutthroat.game.pass');
    case 'points':
      return t('game.moves.points.description', { count: rank ?? '?' });
    case 'scuttle':
      return t('game.moves.scuttle.description');
    case 'royal':
      if (rank) {
        return t(`game.moves.effects[${rank}]`);
      }
      return t('game.moves.royal.description');
    case 'jack':
      return t('game.moves.jack.description');
    case 'joker':
      return t('cutthroat.game.playJoker');
    case 'oneOff':
      if (rank) {
        return t(`game.moves.effects[${rank}]`);
      }
      return t('cutthroat.game.playOneOff');
    case 'discard':
      return t('game.dialogs.fiveDialog.discard');
    case 'counterTwo':
      return t('game.dialogs.counterDialogs.counterTitle');
    case 'counterPass':
      return t('cutthroat.game.counterPassAction');
    case 'resolveThreePick':
      return t('cutthroat.game.pickFromScrap');
    case 'resolveFourDiscard':
      return t('cutthroat.game.discard');
    case 'resolveFiveDiscard':
      return t('cutthroat.game.discard');
    default:
      return '';
  }
}

function formatAction(action) {
  if (!action || !action.type) {return t('cutthroat.game.action');}
  switch (action.type) {
    case 'Draw':
      return t('cutthroat.game.draw');
    case 'Pass':
      return t('cutthroat.game.pass');
    case 'PlayPoints':
      return `${t('cutthroat.game.playPoints')} ${formatCardToken(action.data?.card)}`;
    case 'Scuttle':
      return `${t('cutthroat.game.scuttle')} ${formatCardToken(action.data?.target_point_base)} ${t('cutthroat.game.with')} ${formatCardToken(action.data?.card)}`;
    case 'PlayRoyal':
      return `${t('cutthroat.game.playRoyal')} ${formatCardToken(action.data?.card)}`;
    case 'PlayJack':
      return `${t('cutthroat.game.playJack')} ${formatCardToken(action.data?.jack)} -> ${formatCardToken(action.data?.target_point_base)}`;
    case 'PlayJoker':
      return `${t('cutthroat.game.playJoker')} ${formatCardToken(action.data?.joker)} -> ${formatCardToken(action.data?.target_royal_card)}`;
    case 'PlayOneOff':
      return `${t('cutthroat.game.playOneOff')} ${formatCardToken(action.data?.card)} ${formatOneOffTarget(action.data?.target)}`;
    case 'CounterTwo':
      return `${t('cutthroat.game.counterTwo')} ${formatCardToken(action.data?.two_card)}`;
    case 'CounterPass':
      return t('cutthroat.game.counterPassAction');
    case 'ResolveThreePick':
      return `${t('cutthroat.game.pickFromScrap')} ${formatCardToken(action.data?.card_from_scrap)}`;
    case 'ResolveFourDiscard':
      return `${t('cutthroat.game.discard')} ${formatCardToken(action.data?.card)}`;
    case 'ResolveFiveDiscard':
      return `${t('cutthroat.game.discard')} ${formatCardToken(action.data?.card)}`;
    case 'ResolveSevenChoose':
      return `${t('cutthroat.game.play')} ${formatCardToken(revealToken(action.data?.source_index))} ${formatSevenPlay(action.data?.play)}`;
    default:
      return action.type;
  }
}

function revealToken(index) {
  const tokens = phaseData.value?.revealed_cards ?? [];
  if (typeof index !== 'number') {return '';}
  return tokens[index] ?? '';
}

function formatOneOffTarget(target) {
  if (!target || !target.type) {return '';}
  switch (target.type) {
    case 'None':
      return '';
    case 'Player':
      return `${t('cutthroat.game.targetPlayer')} P${target.data?.seat}`;
    case 'Point':
      return `${t('cutthroat.game.targetPoint')} ${formatCardToken(target.data?.base)}`;
    case 'Royal':
      return `${t('cutthroat.game.targetRoyal')} ${formatCardToken(target.data?.card)}`;
    case 'Jack':
      return `${t('cutthroat.game.targetJack')} ${formatCardToken(target.data?.card)}`;
    case 'Joker':
      return `${t('cutthroat.game.targetJoker')} ${formatCardToken(target.data?.card)}`;
    default:
      return '';
  }
}

function formatSevenPlay(play) {
  if (!play || !play.type) {return '';}
  switch (play.type) {
    case 'Points':
      return t('cutthroat.game.asPoints');
    case 'Scuttle':
      return `${t('cutthroat.game.asScuttle')} ${formatCardToken(play.data?.target)}`;
    case 'Royal':
      return t('cutthroat.game.asRoyal');
    case 'Jack':
      return `${t('cutthroat.game.asJack')} ${formatCardToken(play.data?.target)}`;
    case 'Joker':
      return `${t('cutthroat.game.asJoker')} ${formatCardToken(play.data?.target)}`;
    case 'OneOff':
      return `${t('cutthroat.game.asOneOff')} ${formatOneOffTarget(play.data?.target)}`;
    case 'Discard':
      return t('cutthroat.game.asDiscard');
    default:
      return '';
  }
}

function goToHome() {
  router.push('/');
}

async function handleRematch() {
  if (rematchLoading.value) {return;}
  rematchLoading.value = true;
  try {
    const newGameId = await store.rematchGame(gameId.value);
    await router.push(`/cutthroat/lobby/${newGameId}`);
  } catch (err) {
    snackbarStore.alert(err?.message ?? t('cutthroat.game.actionFailed'));
  } finally {
    rematchLoading.value = false;
  }
}

function scrollHistoryLogs() {
  if (logsContainerDesktop.value) {
    logsContainerDesktop.value.scrollTop = logsContainerDesktop.value.scrollHeight;
  }
  if (logsContainerDrawer.value) {
    logsContainerDrawer.value.scrollTop = logsContainerDrawer.value.scrollHeight;
  }
}

onMounted(async () => {
  setBrowserHeightVariable();
  window.addEventListener('resize', setBrowserHeightVariable);
  await nextTick();
  scrollHistoryLogs();

  try {
    await store.fetchState(gameId.value);
    if (store.status === 0) {
      router.replace(`/cutthroat/lobby/${gameId.value}`);
      return;
    }
  } catch (err) {
    try {
      await store.joinGame(gameId.value);
      await store.fetchState(gameId.value);
      if (store.status === 0) {
        router.replace(`/cutthroat/lobby/${gameId.value}`);
        return;
      }
    } catch (joinErr) {
      snackbarStore.alert(joinErr?.message ?? t('cutthroat.game.loadFailed'));
      router.push('/');
      return;
    }
  }

  store.connectWs(gameId.value);
});

watch(
  () => store.status,
  (status) => {
    if (status === 0) {
      router.replace(`/cutthroat/lobby/${gameId.value}`);
    }
    if (status === 2) {
      clearInteractionState();
    }
  },
);

watch(
  () => legalActions.value,
  () => {
    syncInteractionState();
    const allowedFourTokens = new Set(resolveFourHandCards.value.map((entry) => entry.token));
    selectedResolveFourTokens.value = selectedResolveFourTokens.value.filter((token) => allowedFourTokens.has(token));
    if (selectedResolveFiveToken.value && !resolveFiveDiscardTokens.value.includes(selectedResolveFiveToken.value)) {
      selectedResolveFiveToken.value = null;
    }
  },
  { deep: true },
);

watch(
  () => historyLines.value,
  () => {
    nextTick(() => {
      scrollHistoryLogs();
    });
  },
  { deep: true },
);

watch(
  () => smAndDown.value,
  (isCompact) => {
    if (!isCompact) {
      showHistoryDrawer.value = false;
    }
  },
);

watch(
  () => store.lastError,
  (error) => {
    if (!error) {return;}
    snackbarStore.alert(error.message ?? t('cutthroat.game.actionFailed'));
    store.clearLastError();
    actionInFlight.value = false;
    actionInFlightKey.value = '';
    syncInteractionState();
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

watch(
  () => phaseType.value,
  () => {
    if (isResolvingSeven.value && selectedSource.value?.zone === 'reveal') {
      if (!isRevealSelectable(selectedSource.value.index)) {
        clearInteractionState();
      }
    }
  },
);

onBeforeUnmount(() => {
  window.removeEventListener('resize', setBrowserHeightVariable);
  store.disconnectWs();
});
</script>

<style scoped lang="scss">
@import '@/routes/game/styles/game-ux-shared.scss';

@function bh($quantity) {
  @return calc(var(--browserHeight, 1vh) * #{$quantity});
}

#cutthroat-game-wrapper {
  position: relative;
  height: 100vh;
  height: 100dvh;
  background-image: url('/img/game/board-background.webp');
  background-size: cover;
  background-position: center;
  color: #fff;
  padding: 12px;
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
  overflow-x: hidden;
  overflow-y: auto;
}

.loading {
  text-align: center;
  padding: 40px;
  font-size: 1.2rem;
}

.table {
  display: flex;
  flex-direction: column;
  gap: 12px;
  flex: 1;
  min-height: 0;
}

.mobile-history-controls {
  display: none;
}

.history-toggle-icon {
  cursor: pointer;
  opacity: 0.92;
  transition: opacity 120ms ease;
}

.history-toggle-icon:hover {
  opacity: 1;
}

.finished-subtitle {
  font-size: 1.05rem;
  line-height: 1.4;
  opacity: 0.96;
  text-align: center;
  margin: 4px 0;
}

.finished-actions {
  width: 100%;
  margin-top: 10px;
  display: flex;
  justify-content: center;
  gap: 10px;
  flex-wrap: wrap;
}

.finished-actions .v-btn {
  min-width: 138px;
}

.table-top {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(200px, 300px) minmax(0, 1fr);
  align-items: start;
  gap: 16px;
  min-height: 0;
}

.table-center {
  display: flex;
  justify-content: center;
  gap: 24px;
  align-items: center;
}

.table-bottom {
  display: flex;
  justify-content: center;
  min-height: 0;
}

.player-area {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 16px;
  padding: 16px;
  width: 100%;
  max-width: none;
  transition: border-color 180ms ease, box-shadow 180ms ease, background 180ms ease;
}

.player-area.me {
  max-width: 980px;
}

.player-area.active-turn {
  border-color: rgba(var(--v-theme-accent), 0.7);
  box-shadow: 0 0 0 1px rgba(var(--v-theme-accent), 0.45);
  background: rgba(255, 255, 255, 0.06);
}

.player-header {
  font-family: 'Luckiest Guy', serif;
  font-size: 1.3rem;
  margin-bottom: 8px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.target-player-btn {
  width: 100%;
  border: 0;
  background: transparent;
  text-align: left;
  color: inherit;
  padding: 0;
}

.target-player-btn:disabled {
  cursor: default;
}

.target-player-btn.valid-target {
  color: rgba(var(--v-theme-accent-lighten1));
  cursor: pointer;
}

.turn-status {
  font-family: 'Cormorant Infant', serif;
  font-size: 0.92rem;
  letter-spacing: 0.3px;
  background: rgba(0, 0, 0, 0.32);
  border: 1px solid rgba(255, 255, 255, 0.22);
  border-radius: 999px;
  padding: 2px 10px;
  line-height: 1.2;
  color: rgba(255, 255, 255, 0.92);
}

.turn-status.my-turn {
  background: rgba(255, 255, 255, 0.78);
  border-color: rgba(var(--v-theme-accent), 0.8);
}

.player-hand {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.player-area.opponent .player-hand {
  align-items: flex-start;
}

.player-area.opponent .hand-card {
  flex: 1 1 clamp(50px, 15%, 88px);
  max-width: clamp(50px, 15%, 88px);
  min-width: 42px;
}

.player-area.opponent .hand-card :deep(.player-card) {
  width: 100%;
  max-width: 100%;
  height: auto;
  max-height: clamp(72px, 11vh, 118px);
}

.player-hand.me {
  justify-content: flex-start;
  margin-top: 16px;
  flex-wrap: nowrap;
  overflow-x: auto;
  overflow-y: hidden;
  padding-bottom: 4px;
  min-height: clamp(98px, 15vh, 184px);
  align-items: flex-end;
}

.player-hand.me .hand-card {
  flex: 0 0 auto;
  width: clamp(64px, 9.2vh, 126px);
}

.player-hand.me .hand-card :deep(.player-card) {
  width: 100%;
  max-width: 100%;
}

.player-hand.me.my-turn {
  border: 4px solid rgba(var(--v-theme-accent));
  border-radius: 8px;
  box-shadow:
    0 15px 16px -12px rgba(0, 123, 59, 0.8),
    0 24px 38px 12px rgba(0, 123, 59, 0.8),
    0 10px 50px 16px rgba(33, 150, 83, 0.8);
  background: linear-gradient(0deg, rgba(253, 98, 34, 1), rgba(255, 255, 255, 0.3));
}

.player-hand-targeting-overlay {
  margin-top: 10px;
}

.frozen-zone {
  margin-top: 12px;
}

.player-stacks {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  margin-top: 12px;
}

.stack-group {
  background: rgba(0, 0, 0, 0.2);
  border-radius: 12px;
  padding: 8px;
}

.stack-title {
  font-size: 0.9rem;
  margin-bottom: 6px;
}

.stack-list {
  display: flex;
  flex-direction: row;
  flex-wrap: wrap;
  align-content: flex-start;
  gap: 8px;
}

.stack {
  display: flex;
  flex: 0 0 auto;
  flex-direction: column;
  gap: 2px;
  align-items: flex-start;
}

.stack-card-container {
  position: relative;
  flex: 0 0 auto;
  width: clamp(58px, 8.4vh, 116px);
  overflow: visible;
}

.stack-base {
  width: 100%;
}

.stack-base :deep(.player-card) {
  width: 100%;
  max-width: 100%;
}

.stack-base :deep(.player-card.glasses) {
  width: 100%;
  max-width: 100%;
  height: auto;
}

.stack-meta {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 0.85rem;
}

.stack-controller {
  opacity: 0.8;
  margin-top: 2px;
}

.attachments-overlay {
  position: absolute;
  right: -8%;
  top: 0;
  width: 100%;
  z-index: 2;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
}

.stack-empty {
  font-size: 0.8rem;
  opacity: 0.7;
}

.pile {
  background: rgba(255, 255, 255, 0.06);
  border-radius: 16px;
  padding: 12px 20px;
  min-width: clamp(136px, 17vw, 180px);
  text-align: center;
}

.pile.clickable {
  cursor: pointer;
  border: 1px solid rgba(var(--v-theme-accent-lighten1), 0.45);
}

.pile-title {
  font-size: 0.9rem;
  margin-bottom: 8px;
}

.deck-face {
  width: clamp(68px, 8vw, 90px);
  aspect-ratio: 9 / 13;
  height: auto;
  background: url('/img/cards/card-back.png') center/cover no-repeat;
  border-radius: 8px;
  margin: 0 auto;
}

.reveal-group {
  display: flex;
  gap: 8px;
  justify-content: center;
}

.reveal-card {
  background: transparent;
  border: none;
  padding: 0;
  cursor: default;
}

.reveal-card.clickable {
  cursor: pointer;
}

.reveal-card.selected {
  outline: 2px solid rgba(var(--v-theme-accent-lighten1));
  border-radius: 12px;
}

.history-panel {
  background-color: rgba(241, 200, 160, 0.65);
  color: #111111;
  border-radius: 20px;
  padding: 10px 12px;
  min-height: 0;
}

.history-panel-desktop {
  display: flex;
  flex-direction: column;
  justify-content: flex-start;
  align-self: stretch;
  max-height: 44vh;
}

.history-title {
  font-size: 1.1rem;
  font-weight: 700;
  font-family:
    'Cormorant Infant',
    Century Gothic,
    CenturyGothic,
    AppleGothic,
    sans-serif;
  margin-bottom: 8px;
}

.history-logs {
  max-height: 100%;
  overflow-y: auto;
  overflow-wrap: anywhere;
  font-size: 0.8rem;
  letter-spacing: 0.25px;
  font-family:
    'Libre Baskerville',
    Century Gothic,
    CenturyGothic,
    AppleGothic,
    sans-serif;
}

.history-logs-drawer {
  padding: 12px 16px;
}

.history-log {
  font-size: 0.86rem;
  margin: 0 0 8px;
}

.history-log-empty {
  opacity: 0.75;
}

:deep(#cutthroat-game-over-dialog) {
  backdrop-filter: blur(2px);
}

:deep(#cutthroat-game-over-dialog h1) {
  text-align: center;
  width: 100%;
}

.debug-actions {
  margin-top: 4px;
  background: rgba(0, 0, 0, 0.32);
  border-radius: 10px;
  padding: 8px;
}

.debug-actions summary {
  cursor: pointer;
  margin-bottom: 8px;
  user-select: none;
}

.debug-actions-grid {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.counter-context-error {
  margin: 8px 0;
  padding: 8px 10px;
  border-radius: 8px;
  border: 1px solid rgb(var(--v-theme-error));
  background: rgba(160, 30, 30, 0.2);
  color: rgb(var(--v-theme-on-surface));
  font-size: 0.82rem;
}

:deep(.player-card) {
  max-height: bh(16);
  max-width: calc(bh(16) / 1.45);
}

:deep(.player-card.glasses) {
  max-height: bh(16);
  max-width: calc(bh(16) / 1.45);
  height: auto;
}

.mini-card :deep(.player-card) {
  max-height: bh(6);
  max-width: calc(bh(6) / 1.45);
}

@media (max-width: 1280px) {
  #cutthroat-game-wrapper {
    padding: 10px;
  }

  .table {
    gap: 10px;
  }

  .table-top {
    grid-template-columns: minmax(0, 1fr) minmax(170px, 240px) minmax(0, 1fr);
    gap: 10px;
  }

  .table-center {
    gap: 16px;
  }

  .player-area {
    border-radius: 14px;
    padding: 12px;
  }

  .player-header {
    font-size: 1.12rem;
    margin-bottom: 6px;
  }

  .stack-group {
    padding: 6px;
  }

  .stack-title {
    margin-bottom: 4px;
  }

  .stack-list {
    gap: 6px;
  }

  .stack {
    gap: 6px;
  }

  .history-panel-desktop {
    max-height: 36vh;
  }

  :deep(.player-card) {
    max-height: bh(13);
    max-width: calc(bh(13) / 1.45);
  }

  :deep(.player-card.glasses) {
    max-height: bh(13);
    max-width: calc(bh(13) / 1.45);
    height: auto;
  }

  .mini-card :deep(.player-card) {
    max-height: bh(5);
    max-width: calc(bh(5) / 1.45);
  }
}

@media (max-width: 960px) {
  #cutthroat-game-wrapper {
    height: bh(100);
    padding: 8px;
    overflow: hidden;
  }

  .mobile-history-controls {
    display: flex;
    justify-content: flex-end;
    padding-right: 4px;
    margin-bottom: 2px;
  }

  .table {
    display: grid;
    height: 100%;
    grid-template-rows: minmax(0, 1fr) auto minmax(0, 1fr);
    gap: 8px;
    align-content: stretch;
    overflow: hidden;
  }

  .table.compact-resolving-seven {
    grid-template-rows: minmax(0, 1fr) auto minmax(0, 1fr);
  }

  .table-top {
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 8px;
    align-items: stretch;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .table-center {
    gap: 8px;
    flex-wrap: nowrap;
    min-height: auto;
    align-items: center;
  }

  .history-panel-desktop {
    display: none;
  }

  .player-area {
    padding: 10px;
  }

  .player-area.opponent .player-hand {
    flex-wrap: nowrap;
    overflow-x: auto;
    overflow-y: hidden;
    padding-bottom: 2px;
  }

  .player-area.opponent .hand-card {
    flex: 0 0 clamp(34px, 8.2vw, 52px);
    max-width: clamp(34px, 8.2vw, 52px);
    min-width: 30px;
  }

  .player-area.opponent .player-stacks {
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }

  .player-area.me .player-stacks {
    grid-template-columns: 1fr 1fr;
  }

  .stack-controller {
    display: none;
  }

  .player-area.opponent .stack-list {
    max-height: clamp(46px, 8vh, 80px);
    overflow-y: auto;
    padding-right: 2px;
  }

  .player-area.opponent .stack {
    align-items: center;
  }

  .table-bottom {
    align-items: stretch;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .player-area.me {
    display: flex;
    flex-direction: column;
    justify-content: flex-start;
    max-height: none;
  }

  .stack-title {
    font-size: 0.8rem;
  }

  .stack-meta {
    font-size: 0.76rem;
  }

  .stack-card-container {
    width: clamp(34px, 5.6vh, 54px);
  }

  .player-hand.me {
    min-height: clamp(70px, 10vh, 108px);
  }

  .player-hand.me .hand-card {
    width: clamp(48px, 7vh, 78px);
  }

  .attachments-overlay {
    right: 0;
  }

  .pile {
    min-width: clamp(108px, 22vw, 136px);
    padding: 6px 8px;
  }

  .pile-title {
    font-size: 0.78rem;
    margin-bottom: 6px;
  }

  :deep(.player-card) {
    max-height: bh(8.8);
    max-width: calc(bh(8.8) / 1.45);
  }

  :deep(.player-card.glasses) {
    max-height: bh(8.8);
    max-width: calc(bh(8.8) / 1.45);
    height: auto;
  }

  .mini-card :deep(.player-card) {
    max-height: bh(3.8);
    max-width: calc(bh(3.8) / 1.45);
  }

  .debug-actions {
    display: none;
  }
}

@media (max-width: 600px) {
  #cutthroat-game-wrapper {
    padding: 6px;
  }

  .table {
    grid-template-rows: minmax(0, 1fr) auto minmax(0, 1fr);
    gap: 6px;
  }

  .table.compact-resolving-seven {
    grid-template-rows: minmax(0, 1fr) auto minmax(0, 1fr);
  }

  .mobile-history-controls {
    margin-bottom: 0;
  }

  .table-top {
    align-items: stretch;
    gap: 6px;
    padding-right: 1px;
  }

  .table-center {
    gap: 6px;
    min-height: auto;
    align-items: center;
  }

  .table.compact-resolving-seven .table-center {
    min-height: auto;
  }

  .player-area {
    padding: 6px;
    border-radius: 12px;
  }

  .player-area.opponent {
    min-width: 0;
    max-width: none;
  }

  .player-area.opponent .hand-card {
    flex-basis: clamp(30px, 18vw, 46px);
    max-width: clamp(30px, 18vw, 46px);
    min-width: 26px;
  }

  .player-area.me {
    max-width: none;
    min-height: 0;
    overflow: hidden;
  }

  .player-header {
    font-size: 0.82rem;
    margin-bottom: 4px;
  }

  .turn-status {
    font-size: 0.7rem;
    padding: 1px 7px;
  }

  .player-hand {
    gap: 2px;
  }

  .player-hand.me {
    margin-top: 4px;
    justify-content: flex-start;
    flex-wrap: nowrap;
    overflow-x: auto;
    min-height: clamp(62px, 9vh, 92px);
  }

  .player-hand.me .hand-card {
    width: clamp(40px, 6.8vh, 58px);
  }

  .player-area.opponent .player-stacks {
    display: flex;
    grid-template-columns: none;
    gap: 4px;
    margin-top: 4px;
  }

  .player-area.opponent .stack-group {
    min-width: 0;
    flex: 1;
  }

  .player-area.opponent .stack-list {
    max-height: clamp(34px, 6.2vh, 58px);
  }

  .player-stacks {
    margin-top: 4px;
    gap: 4px;
    min-height: 0;
  }

  .player-area.me .player-stacks {
    grid-template-columns: 1fr 1fr;
  }

  .stack-group {
    padding: 4px;
  }

  .stack-title {
    font-size: 0.7rem;
    margin-bottom: 2px;
  }

  .stack-list {
    gap: 2px;
  }

  .stack {
    gap: 2px;
  }

  .stack-meta {
    gap: 1px;
    font-size: 0.62rem;
  }

  .stack-card-container {
    width: clamp(26px, 4.6vh, 40px);
  }

  .pile {
    min-width: 0;
    padding: 6px 8px;
  }

  .pile-title {
    font-size: 0.68rem;
    margin-bottom: 4px;
  }

  .deck-face {
    width: 44px;
    height: 63px;
  }

  .reveal-group {
    gap: 4px;
  }

  .reveal-group .hand-card {
    width: clamp(30px, 4.7vh, 42px);
  }

  .reveal-group .hand-card :deep(.player-card) {
    max-height: bh(5.5);
    max-width: calc(bh(5.5) / 1.45);
  }

  .frozen-zone {
    margin-top: 4px;
  }

  :deep(.player-card) {
    max-height: bh(6.2);
    max-width: calc(bh(6.2) / 1.45);
  }

  :deep(.player-card.glasses) {
    max-height: bh(6.2);
    max-width: calc(bh(6.2) / 1.45);
    height: auto;
  }

  .mini-card :deep(.player-card) {
    max-height: bh(2.8);
    max-width: calc(bh(2.8) / 1.45);
  }
}
</style>
