<template>
  <div
    class="cutthroat-card"
    :class="{
      clickable,
      selected: isSelected,
      'valid-target': isValidTarget,
      frozen: isFrozen,
    }"
    :role="clickable ? 'button' : undefined"
    @click="handleClick"
  >
    <GameCard
      :rank="resolvedRank"
      :suit="resolvedSuit"
      :class="{ 'is-hidden': card.kind === 'hidden' }"
      :is-selected="isSelected"
      :is-valid-target="isValidTarget"
      :is-glasses="isGlasses"
      :is-frozen="isFrozen"
      :is-jack="isJack"
      :controlled-by="controlledBy"
      :scuttled-by="scuttledBy"
      :high-elevation="highElevation"
    />
  </div>
</template>

<script setup>
import { computed } from 'vue';
import GameCard from '@/routes/game/components/GameCard.vue';

const props = defineProps({
  card: {
    type: Object,
    required: true,
  },
  isSelected: {
    type: Boolean,
    default: false,
  },
  isValidTarget: {
    type: Boolean,
    default: false,
  },
  isGlasses: {
    type: Boolean,
    default: false,
  },
  isFrozen: {
    type: Boolean,
    default: false,
  },
  clickable: {
    type: Boolean,
    default: false,
  },
  isJack: {
    type: Boolean,
    default: false,
  },
  controlledBy: {
    type: String,
    default: '',
  },
  scuttledBy: {
    type: Object,
    default: null,
  },
  highElevation: {
    type: Boolean,
    default: false,
  },
});

const emit = defineEmits([ 'click' ]);

const resolvedRank = computed(() => (props.card.kind === 'joker' ? 14 : props.card.rank));
const resolvedSuit = computed(() => (props.card.kind === 'joker' ? (props.card.id ?? 0) : props.card.suit));

function handleClick() {
  if (!props.clickable) {return;}
  emit('click');
}
</script>

<style scoped lang="scss">
.cutthroat-card {
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 12px;

  &.clickable {
    cursor: pointer;
  }
}

</style>
