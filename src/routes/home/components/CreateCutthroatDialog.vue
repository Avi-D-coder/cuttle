<template>
  <BaseDialog
    :id="`create-cutthroat-dialog`"
    v-model="show"
    :title="t('cutthroat.lobby.createTitle')"
    :opacity="1"
    data-cy="create-cutthroat-dialog"
  >
    <template #activator>
      <v-btn
        class="px-16 w-100"
        color="primary"
        size="x-large"
        text-color="white"
        data-cy="create-cutthroat-btn"
      >
        {{ t('cutthroat.lobby.createButton') }}
      </v-btn>
    </template>
    <template #actions>
      <v-btn
        class="mr-2"
        :disabled="loading"
        variant="text"
        color="surface-1"
        @click="cancel"
      >
        {{ t('global.cancel') }}
      </v-btn>
      <v-btn
        :loading="loading"
        color="surface-1"
        variant="flat"
        @click="submit"
      >
        {{ t('cutthroat.lobby.createButton') }}
      </v-btn>
    </template>
  </BaseDialog>
</template>

<script setup>
import { ref } from 'vue';
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import BaseDialog from '@/components/BaseDialog.vue';
import { useCutthroatStore } from '@/stores/cutthroat';
import { useSnackbarStore } from '@/stores/snackbar';

const { t } = useI18n();
const router = useRouter();
const store = useCutthroatStore();
const snackbarStore = useSnackbarStore();

const show = ref(false);
const loading = ref(false);

async function submit() {
  loading.value = true;
  try {
    const id = await store.createGame();
    show.value = false;
    router.push(`/cutthroat/lobby/${id}`);
  } catch (err) {
    snackbarStore.alert(err?.message ?? t('cutthroat.lobby.joinFailed'));
  } finally {
    loading.value = false;
  }
}

function cancel() {
  show.value = false;
}
</script>
