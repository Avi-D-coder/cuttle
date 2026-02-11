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

          <template v-else>
            <v-list-item
              v-if="canRequestStalemate"
              data-cy="stalemate-initiate"
              prepend-icon="mdi-handshake"
              @click="shownDialog = 'stalemate'"
            >
              {{ t('game.menus.gameMenu.stalemate') }}
            </v-list-item>
          </template>

          <TheLanguageSelector />

          <v-list-item data-cy="refresh" prepend-icon="mdi-refresh" @click="refreshPage">
            {{ t('game.menus.gameMenu.refresh') }}
          </v-list-item>
        </v-list>
      </template>
    </BaseMenu>

    <RulesDialog v-model="showRulesDialog" @open="closeMenu" @close="closeDialog" />

    <BaseDialog
      id="request-gameover-dialog"
      v-model="showStalemateDialog"
      :title="t('game.menus.gameMenu.stalemate')"
    >
      <template #body>
        <p class="pt-4 pb-8">
          {{ t('game.menus.gameMenu.stalemateDialog') }}
        </p>
      </template>

      <template #actions>
        <v-btn
          data-cy="request-gameover-cancel"
          :disabled="loading"
          variant="outlined"
          color="surface-1"
          class="mr-4"
          @click="closeDialog"
        >
          {{ t('game.menus.gameMenu.cancel') }}
        </v-btn>
        <v-btn
          variant="flat"
          data-cy="request-gameover-confirm"
          color="error"
          :loading="loading"
          @click="requestStalemate"
        >
          {{ t('game.menus.gameMenu.stalemate') }}
        </v-btn>
      </template>
    </BaseDialog>
  </div>
</template>

<script setup>
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseMenu from '@/components/BaseMenu.vue';
import BaseDialog from '@/components/BaseDialog.vue';
import TheLanguageSelector from '@/components/TheLanguageSelector.vue';
import RulesDialog from '@/routes/game/components/dialogs/components/RulesDialog.vue';

const componentProps = defineProps({
  isSpectating: {
    type: Boolean,
    required: true,
  },
  canRequestStalemate: {
    type: Boolean,
    default: false,
  },
});

const emit = defineEmits([ 'go-home', 'request-stalemate' ]);
const { t } = useI18n();

const showGameMenu = ref(false);
const shownDialog = ref('');
const loading = ref(false);

const showRulesDialog = computed({
  get() {
    return shownDialog.value === 'rules';
  },
  set(val) {
    shownDialog.value = val ? 'rules' : '';
  }
});

const showStalemateDialog = computed({
  get() {
    return shownDialog.value === 'stalemate';
  },
  set(val) {
    shownDialog.value = val ? 'stalemate' : '';
    if (!val) {
      showGameMenu.value = false;
    }
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

function requestStalemate() {
  if (!componentProps.canRequestStalemate || loading.value) {return;}
  loading.value = true;
  try {
    emit('request-stalemate');
  } finally {
    loading.value = false;
    shownDialog.value = '';
    showGameMenu.value = false;
  }
}
</script>
