<template>
  <v-overlay
    id="cutthroat-move-choice-overlay"
    class="d-flex flex-column justify-center align-center"
    :model-value="modelValue"
    @click="$emit('cancel')"
  >
    <div id="close-wrapper" class="d-flex justify-end my-4">
      <v-btn
        icon
        variant="text"
        color="surface-2"
        size="x-large"
        aria-label="Cancel Move"
        @click="$emit('cancel')"
      >
        <v-icon
          icon="mdi-close"
          size="x-large"
          aria-hidden="true"
        />
      </v-btn>
    </div>
    <div v-if="selectedCard" class="d-flex justify-center">
      <CutthroatCard
        :card="selectedCard"
        :is-frozen="isFrozen"
      />
    </div>
    <div id="options-wrapper" class="d-flex justify-space-between my-4">
      <MoveChoiceCard
        v-for="choice in moveChoices"
        :key="choice.type"
        :move-name="choice.displayName"
        :move-description="choice.moveDescription"
        :event-name="choice.type"
        :disabled="disabled || !!choice.disabled"
        :disabled-explanation="choice.disabledExplanation || ''"
        :card-width="cardWidth"
        :data-cy="`cutthroat-move-choice-${choice.type}`"
        @choose-move="$emit('choose-move', choice.type)"
      />
    </div>
  </v-overlay>
</template>

<script>
import MoveChoiceCard from '@/routes/game/components/MoveChoiceCard.vue';
import CutthroatCard from '@/routes/cutthroat/components/CutthroatCard.vue';

export default {
  name: 'CutthroatMoveChoiceOverlay',
  components: {
    MoveChoiceCard,
    CutthroatCard,
  },
  props: {
    modelValue: {
      type: Boolean,
      required: true,
    },
    selectedCard: {
      type: Object,
      default: null,
    },
    isFrozen: {
      type: Boolean,
      default: false,
    },
    moveChoices: {
      type: Array,
      default: () => [],
    },
    disabled: {
      type: Boolean,
      default: false,
    },
  },
  emits: [ 'choose-move', 'cancel' ],
  computed: {
    cardWidth() {
      if (this.$vuetify.display.smAndDown) {
        return '100%';
      }
      switch (this.moveChoices.length) {
        case 1:
          return '100%';
        case 2:
          return '50%';
        case 3:
        default:
          return '30%';
      }
    },
  },
};
</script>

<style scoped lang="scss">
#cutthroat-move-choice-overlay {
  & #close-wrapper {
    width: 85%;
  }

  & #options-wrapper {
    width: 85%;
    max-width: 1300px;
  }
}

@media (max-width: 900px) {
  #options-wrapper {
    flex-direction: column;
  }
}
</style>
