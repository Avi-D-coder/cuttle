<template>
  <div>
    <BaseMenu v-model="showGameMenu">
      <template #activator="{ props }">
        <v-btn
          id="game-menu-activator"
          class="ml-0"
          v-bind="{ ...props }"
          icon
          variant="text"
          aria-label="Open Game Menu"
        >
          <v-icon color="neutral-lighten-2" icon="mdi-cog" aria-hidden="true" />
        </v-btn>
      </template>

      <template #body="{ listProps }">
        <v-list id="game-menu" v-bind="listProps">
          <v-list-item data-cy="rules-open" prepend-icon="mdi-information" @click="shownDialog = 'rules'">
            {{ t('game.menus.gameMenu.rules') }}
          </v-list-item>

          <v-list-item
            v-if="isSpectating"
            data-cy="stop-spectating"
            prepend-icon="mdi-home"
            @click.stop="goHome"
          >
            {{ t('game.menus.gameMenu.home') }}
          </v-list-item>

          <TheLanguageSelector />

          <v-list-item data-cy="refresh" prepend-icon="mdi-refresh" @click="refreshPage">
            {{ t('game.menus.gameMenu.refresh') }}
          </v-list-item>
        </v-list>
      </template>
    </BaseMenu>

    <RulesDialog v-model="showRulesDialog" @open="closeMenu" @close="closeDialog" />
  </div>
</template>

<script setup>
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseMenu from '@/components/BaseMenu.vue';
import TheLanguageSelector from '@/components/TheLanguageSelector.vue';
import RulesDialog from '@/routes/game/components/dialogs/components/RulesDialog.vue';

defineProps({
  isSpectating: {
    type: Boolean,
    required: true,
  },
});

const emit = defineEmits([ 'go-home' ]);
const { t } = useI18n();

const showGameMenu = ref(false);
const shownDialog = ref('');

const showRulesDialog = computed({
  get() {
    return shownDialog.value === 'rules';
  },
  set(val) {
    shownDialog.value = val ? 'rules' : '';
  }
});

function closeMenu() {
  showGameMenu.value = false;
}

function closeDialog() {
  shownDialog.value = '';
}

function goHome() {
  closeMenu();
  emit('go-home');
}

function refreshPage() {
  window.location.reload();
}
</script>
