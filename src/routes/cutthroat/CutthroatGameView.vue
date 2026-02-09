<template>
  <div id="cutthroat-game-wrapper">
    <div v-if="!playerView" class="loading">
      {{ t('cutthroat.game.loading') }}
    </div>

    <template v-else>
      <div
        id="game-menu-wrapper"
        class="cutthroat-top-controls d-flex align-center justify-end"
        :style="menuWrapperStyle"
      >
        <CutthroatGameMenu :is-spectating="isSpectatorMode" @go-home="goToHome" />
        <v-icon
          v-if="smAndDown"
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
            <p v-if="spectatorNames.length > 0" class="history-spectators">
              {{ t('game.menus.spectatorListMenu.spectators') }}: {{ spectatorNames.join(', ') }}
            </p>
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
                :clickable="card.isKnown && (!isActionDisabled || isSpectatorMode)"
                :is-selected="isHandSourceSelected(card)"
                :is-frozen="isFrozenToken(card.token)"
                :data-cutthroat-hand-card="card.token"
                @click="handleHandCardClick(card)"
              />
            </div>
          </div>
        </div>

        <details v-if="showDebugActions" class="debug-actions">
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
          :disabled="isSpectatorMode || !canSubmitResolveFour"
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
          :disabled="isSpectatorMode || !canSubmitResolveFive"
          @click="submitResolveFiveDiscard"
        >
          {{ resolveFiveDialogButton }}
        </v-btn>
      </template>
    </BaseDialog>

    <BaseDialog
      id="cutthroat-game-over-dialog"
      :model-value="showGameOverDialog"
      :title="t('cutthroat.game.gameOverTitle')"
      :persistent="true"
      :max-width="560"
    >
      <template #body>
        <div class="finished-subtitle">
          {{ gameResultText }}
        </div>
        <div
          v-if="!isSpectatorMode && rematchOfferPending"
          class="finished-subtitle"
          data-cy="cutthroat-rematch-waiting"
        >
          {{ t('game.dialogs.gameOverDialog.matchStatus.waitingForPlayers') }}
        </div>
      </template>
      <template #actions>
        <div class="finished-actions">
          <v-btn
            v-if="!isSpectatorMode"
            size="small"
            color="primary"
            variant="flat"
            :loading="rematchLoading"
            data-cy="cutthroat-rematch-btn"
            @click="handleRematch"
          >
            {{ rematchButtonText }}
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

    <CutthroatPlaybackControls
      v-if="showPlaybackControls"
      :game-id="gameId"
      :state-count="replayStateCount"
    />
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, ref, watch } from 'vue';
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
import CutthroatPlaybackControls from '@/routes/cutthroat/components/CutthroatPlaybackControls.vue';
import CutthroatGameMenu from '@/routes/cutthroat/components/CutthroatGameMenu.vue';
import { parseCardToken, formatCardToken } from '@/util/cutthroat-cards';
import {
  deriveFallbackChoiceTypesForSelectedCard,
  extractActionSource,
  getCutthroatGameResult,
  isActionInteractionDisabled,
  isCutthroatGameFinished,
  parseTokenlogActions,
  shouldShowCutthroatGameOverDialog,
} from '@/routes/cutthroat/helpers';
import { useCutthroatSeatData } from '@/routes/cutthroat/composables/useCutthroatSeatData';
import { useCutthroatInteractions } from '@/routes/cutthroat/composables/useCutthroatInteractions';
import { useCutthroatLifecycle } from '@/routes/cutthroat/composables/useCutthroatLifecycle';

const route = useRoute();
const router = useRouter();
const { t } = useI18n();
const { smAndDown } = useDisplay();
const store = useCutthroatStore();
const snackbarStore = useSnackbarStore();

const isDevMode = import.meta.env.DEV;
const showDebugActions = computed(() => {
  if (!isDevMode || typeof window === 'undefined') {return false;}
  return window.CUTTHROAT_DEBUG_ACTIONS === true || window.cuttle?.showCutthroatDebugActions === true;
});

const gameId = computed(() => Number(route.params.gameId));
const playerView = computed(() => store.playerView);
const phase = computed(() => playerView.value?.phase ?? null);
const phaseType = computed(() => phase.value?.type ?? null);
const phaseData = computed(() => phase.value?.data ?? {});
const legalActions = computed(() => store.legalActions ?? []);
const historyLines = computed(() => store.logTail ?? []);
const seatEntries = computed(() => store.lobby?.seats ?? []);
const spectatorNames = computed(() => store.spectatingUsers ?? []);
const isSpectatorMode = computed(() => store.isSpectator);
const isSpectateRoute = computed(() => route.name === 'CutthroatSpectate');
const replayStateIndex = computed(() => {
  const raw = Number(route.query.gameStateIndex);
  return Number.isInteger(raw) && raw >= -1 ? raw : -1;
});
const hasReplayStateIndexQuery = computed(() => {
  return Object.prototype.hasOwnProperty.call(route.query, 'gameStateIndex');
});
const replayStateCount = computed(() => {
  try {
    return parseTokenlogActions(store.tokenlog).length + 1;
  } catch (_) {
    return 1;
  }
});
const showPlaybackControls = computed(() => {
  return isSpectateRoute.value
    && replayStateCount.value > 1
    && (isCutthroatGameFinished(store.status) || replayStateIndex.value >= 0);
});
const menuWrapperStyle = computed(() => {
  return {
    zIndex: isSpectatorMode.value ? 2411 : 3,
  };
});

const mySeat = computed(() => store.seat ?? 0);
const activeTurnSeat = computed(() => playerView.value?.turn ?? null);

const showHistoryDrawer = ref(false);
const logsContainerDesktop = ref(null);
const logsContainerDrawer = ref(null);
const rematchLoading = ref(false);
const rematchLobbyId = ref(null);
const rematchOfferPending = ref(false);
let rematchLobbyWsConnected = false;

const isMainPhase = computed(() => phaseType.value === 'Main');
const isCounteringPhase = computed(() => phaseType.value === 'Countering');
const isResolvingThree = computed(() => phaseType.value === 'ResolvingThree');
const isResolvingFour = computed(() => phaseType.value === 'ResolvingFour');
const isResolvingFive = computed(() => phaseType.value === 'ResolvingFive');
const isResolvingSeven = computed(() => phaseType.value === 'ResolvingSeven');
const isFinished = computed(() => isCutthroatGameFinished(store.status));
const showGameOverDialog = computed(() => {
  return shouldShowCutthroatGameOverDialog({
    status: store.status,
    isSpectateRoute: isSpectateRoute.value,
    hasReplayStateIndexQuery: hasReplayStateIndexQuery.value,
    replayStateIndex: replayStateIndex.value,
    replayStateCount: replayStateCount.value,
  });
});
const rematchButtonText = computed(() => {
  return rematchOfferPending.value
    ? t('cutthroat.lobby.unready')
    : t('game.dialogs.gameOverDialog.rematch');
});
const rematchLobbyIsOpen = computed(() => {
  if (!rematchLobbyId.value) {return false;}
  return store.lobbies.some((lobby) => lobby.id === rematchLobbyId.value);
});
const rematchGameHasStarted = computed(() => {
  if (!rematchLobbyId.value) {return false;}
  return store.spectateGames.some((game) => game.id === rematchLobbyId.value);
});

const localHandActionTokens = computed(() => {
  // Spectator/replay already has full hand visibility in spectator_view.
  // Avoid inferring cards from legal actions because those actions can be
  // generated from another seat's replay perspective.
  if (isSpectatorMode.value) {
    return [];
  }
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

const {
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
} = useCutthroatSeatData({
  playerView,
  phaseData,
  isResolvingSeven,
  mySeat,
  seatEntries,
  localHandActionTokens,
});

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

const isActionDisabled = computed(() => {
  return isActionInteractionDisabled(store.status, actionInFlight.value, isSpectatorMode.value);
});

const {
  actionInFlight,
  actionInFlightKey,
  selectedSource,
  selectedChoice,
  selectedResolveFourTokens,
  selectedResolveFiveToken,
  selectedSourceChoices,
  isTargeting,
  playerTargetChoices,
  showFourPlayerTargetDialog,
  showMoveChoiceOverlay,
  isResolvingThreeTurn,
  counterDialogOneOff,
  counterDialogTarget,
  counterDialogTwosPlayed,
  counterDialogTwosInHand,
  showCounterDialog,
  showCannotCounterDialog,
  counterDialogInvariantError,
  canUseDeck,
  selectedSourceCard,
  selectedSourceIsFrozen,
  resolveFiveDiscardTokens,
  showResolveFourDialog,
  showResolveFiveDialog,
  resolveFourHandCards,
  resolveFiveHandCards,
  canSubmitResolveFour,
  canSubmitResolveFive,
  isActionLoading,
  clearInteractionState,
  cancelTargeting,
  syncInteractionState,
  isFrozenToken,
  isHandSourceSelected,
  isRevealSelected,
  isPointTarget,
  isRoyalTarget,
  isJackTarget,
  isJokerTarget,
  isPlayerTarget,
  sendResolvedAction,
  chooseMove,
  handleDeckClick,
  handleCounterPass,
  handleCounterTwoFromDialog,
  toggleResolveFourCard,
  submitResolveFourDiscard,
  submitResolveFiveDiscard,
  handleHandCardClick,
  handleRevealClick,
  handleScrapCardClick,
  handleRequestScrapStraighten,
  handlePointTargetClick,
  handleRoyalTargetClick,
  handleJackTargetClick,
  handleJokerTargetClick,
  handlePlayerTargetClick,
  isRevealSelectable,
} = useCutthroatInteractions({
  store,
  snackbarStore,
  t,
  legalActions,
  phaseType,
  isActionDisabled,
  isFinished,
  isMainPhase,
  isCounteringPhase,
  isResolvingThree,
  isResolvingFour,
  isResolvingFive,
  myHandCards,
  myFrozenTokens,
  revealedCardEntries,
  isSpectatorMode,
  replayStateIndex,
  localHandActionTokens,
  cardTokenToDialogCard,
});

const isMyTurn = computed(() => !isFinished.value && activeTurnSeat.value === mySeat.value);

const moveChoiceCards = computed(() => {
  const legalChoices = isSpectatorMode.value ? [] : selectedSourceChoices.value;
  if (legalChoices.length > 0) {
    return legalChoices.map((choice) => ({
      type: choice.type,
      displayName: formatChoiceType(choice.type),
      moveDescription: describeChoice(choice.type),
    }));
  }

  const fallbackTypes = deriveFallbackChoiceTypesForSelectedCard(
    selectedSource.value,
    selectedSourceCard.value,
  );
  const disabledExplanation = fallbackDisabledExplanation();
  return fallbackTypes.map((choiceType) => ({
    type: choiceType,
    displayName: formatChoiceType(choiceType),
    moveDescription: describeChoice(choiceType),
    disabled: true,
    disabledExplanation,
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
  if (isSpectatorMode.value) {return t('cutthroat.game.spectating');}
  return playerView.value.turn === mySeat.value ? t('game.turn.yourTurn') : t('game.turn.opponentTurn');
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

function isActiveTurnSeat(seat) {
  if (isFinished.value) {return false;}
  return activeTurnSeat.value === seat;
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
      return t('cutthroat.game.jokerDescription');
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

function fallbackDisabledExplanation() {
  if (!isMyTurn.value) {return t('game.moves.disabledMove.notTurn');}
  if (selectedSourceIsFrozen.value) {return t('game.moves.disabledMove.frozenCard');}
  return t('cutthroat.game.waiting');
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

function scrollHistoryLogs() {
  if (logsContainerDesktop.value) {
    logsContainerDesktop.value.scrollTop = logsContainerDesktop.value.scrollHeight;
  }
  if (logsContainerDrawer.value) {
    logsContainerDrawer.value.scrollTop = logsContainerDrawer.value.scrollHeight;
  }
}

function stopRematchLobbyWatch() {
  if (!rematchLobbyWsConnected) {return;}
  store.disconnectLobbyWs();
  rematchLobbyWsConnected = false;
}

function startRematchLobbyWatch() {
  if (rematchLobbyWsConnected) {return;}
  store.connectLobbyWs();
  rematchLobbyWsConnected = true;
}

async function goToHome() {
  if (rematchOfferPending.value && rematchLobbyId.value) {
    try {
      await store.setReady(rematchLobbyId.value, false);
    } catch (_) {
      // ignore cancellation errors when leaving
    }
    rematchOfferPending.value = false;
  }
  stopRematchLobbyWatch();
  router.push('/');
}

async function handleRematch() {
  if (isSpectatorMode.value) {return;}
  if (rematchLoading.value) {return;}
  rematchLoading.value = true;
  try {
    if (rematchOfferPending.value && rematchLobbyId.value) {
      await store.setReady(rematchLobbyId.value, false);
      rematchOfferPending.value = false;
      stopRematchLobbyWatch();
      return;
    }
    const newGameId = await store.rematchGame(gameId.value);
    rematchLobbyId.value = newGameId;
    await store.setReady(newGameId, true);
    rematchOfferPending.value = true;
    startRematchLobbyWatch();
  } catch (err) {
    snackbarStore.alert(err?.message ?? t('cutthroat.game.actionFailed'));
  } finally {
    rematchLoading.value = false;
  }
}

watch(
  () => rematchOfferPending.value,
  (pending) => {
    if (pending && !isSpectatorMode.value) {
      startRematchLobbyWatch();
      return;
    }
    stopRematchLobbyWatch();
  },
);

watch(
  () => rematchGameHasStarted.value,
  async (started) => {
    if (!started || !rematchOfferPending.value || !rematchLobbyId.value) {return;}
    const nextGameId = rematchLobbyId.value;
    rematchOfferPending.value = false;
    rematchLobbyId.value = null;
    stopRematchLobbyWatch();
    store.disconnectWs();
    await store.fetchState(nextGameId);
    store.connectWs(nextGameId);
    await router.push(`/cutthroat/game/${nextGameId}`);
  },
);

watch(
  () => rematchLobbyId.value,
  (id) => {
    if (!id || !rematchOfferPending.value) {return;}
    if (rematchLobbyIsOpen.value || rematchGameHasStarted.value) {return;}
    // Initial lobby snapshot can arrive asynchronously after rematch creation.
    startRematchLobbyWatch();
  },
);

useCutthroatLifecycle({
  store,
  router,
  t,
  snackbarStore,
  gameId,
  isSpectateRoute,
  isSpectatorMode,
  replayStateIndex,
  legalActions,
  resolveFourHandCards,
  selectedResolveFourTokens,
  resolveFiveDiscardTokens,
  selectedResolveFiveToken,
  syncInteractionState,
  historyLines,
  scrollHistoryLogs,
  smAndDown,
  showHistoryDrawer,
  clearInteractionState,
  actionInFlight,
  actionInFlightKey,
  phaseType,
  isResolvingSeven,
  selectedSource,
  isRevealSelectable,
});

onBeforeUnmount(() => {
  stopRematchLobbyWatch();
});
</script>
<style scoped lang="scss">
@use '@/routes/game/styles/game-ux-shared.scss' as *;

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

.cutthroat-top-controls {
  gap: 6px;
  margin-bottom: 8px;
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
  flex: 0 0 auto;
  min-height: 0;
}

.table-center {
  display: flex;
  flex: 1 1 auto;
  justify-content: center;
  gap: 24px;
  align-items: center;
  min-height: 0;
}

.table-bottom {
  display: flex;
  flex: 0 0 auto;
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

.player-area.opponent {
  align-self: start;
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

.history-spectators {
  margin-bottom: 8px;
  font-size: 0.85rem;
  font-weight: 600;
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
  max-height: 6vh;
  max-width: calc(6vh / 1.45);
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
    max-height: 5vh;
    max-width: calc(5vh / 1.45);
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
    grid-template-rows: minmax(0, 1fr) auto auto;
    gap: 8px;
    align-content: stretch;
    overflow: hidden;
  }

  .table.compact-resolving-seven {
    grid-template-rows: minmax(0, 1fr) auto auto;
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
    max-height: 3.8vh;
    max-width: calc(3.8vh / 1.45);
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
    grid-template-rows: minmax(0, 1fr) auto auto;
    gap: 6px;
  }

  .table.compact-resolving-seven {
    grid-template-rows: minmax(0, 1fr) auto auto;
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
    max-height: 2.8vh;
    max-width: calc(2.8vh / 1.45);
  }
}
</style>
