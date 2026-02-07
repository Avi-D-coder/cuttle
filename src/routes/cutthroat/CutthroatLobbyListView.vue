<template>
  <v-container>
    <h1 class="mb-4">
      {{ t('cutthroat.lobby.listTitle') }}
    </h1>
    <v-btn
      class="px-16"
      color="primary"
      size="x-large"
      text-color="white"
      data-cy="create-cutthroat-btn"
      @click="createCutthroatGame"
    >
      {{ t('cutthroat.lobby.createButton') }}
    </v-btn>
    <v-divider class="my-6" />
    <p v-if="lobbies.length === 0">
      {{ t('cutthroat.lobby.noLobbies') }}
    </p>
    <CutthroatLobbyListItem
      v-for="lobby in lobbies"
      :key="lobby.id"
      :lobby="lobby"
    />
  </v-container>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { useCutthroatStore } from '@/stores/cutthroat';
import CutthroatLobbyListItem from '@/routes/home/components/CutthroatLobbyListItem.vue';
import { useSnackbarStore } from '@/stores/snackbar';

const { t } = useI18n();
const router = useRouter();
const store = useCutthroatStore();
const snackbarStore = useSnackbarStore();
const lobbies = computed(() => store.lobbies ?? []);

async function createCutthroatGame() {
  try {
    const gameId = await store.createGame();
    router.push(`/cutthroat/lobby/${gameId}`);
  } catch (err) {
    snackbarStore.alert(err?.message ?? t('cutthroat.lobby.joinFailed'));
  }
}

onMounted(() => {
  store.connectLobbyWs();
});

onBeforeUnmount(() => {
  store.disconnectLobbyWs();
});
</script>
