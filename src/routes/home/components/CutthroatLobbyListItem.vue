<template>
  <div>
    <v-row class="list-item" data-cy="cutthroat-list-item">
      <v-col lg="6" class="list-item__inner-text">
        <p class="game-name text-surface-1">
          {{ lobby.name || `Cutthroat #${lobby.id}` }}
        </p>
        <p class="text-surface-1">
          {{ lobby.seat_count ?? 0 }} / 3 {{ t('home.players') }}
        </p>
      </v-col>
      <v-col lg="6" class="list-item__button pr-md-0">
        <v-btn
          class="w-100"
          color="surface-1"
          variant="outlined"
          min-width="200"
          :disabled="lobby.seat_count >= 3"
          @click="joinLobby"
        >
          <v-icon
            class="mr-4"
            size="medium"
            icon="mdi-account-group"
            aria-hidden="true"
          />
          {{ t('cutthroat.lobby.join') }}
        </v-btn>
      </v-col>
    </v-row>
    <v-divider color="surface-1" class="mb-4 mx-2 border-opacity-100 px-5" />
  </div>
</template>

<script setup>
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';

const props = defineProps({
  lobby: {
    type: Object,
    required: true,
  },
});

const router = useRouter();
const { t } = useI18n();

function joinLobby() {
  router.push(`/cutthroat/lobby/${props.lobby.id}`);
}
</script>

<style scoped lang="scss">
.list-item {
  margin: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding-top: 0.5rem;
  word-break: break-all;
  & .game-name {
    font-weight: 600;
    font-size: 1.5em;
    text-align: left;
    padding-right: 1rem;
  }
  & p {
    line-height: 1;
    margin: 3px auto;
  }
  &__inner-text {
    align-items: center;
    padding-bottom: 1rem;
    padding-top: 0.25rem;
  }
  &__button {
    display: flex;
    align-items: center;
    justify-content: end;
    margin-top: 0;
    padding-top: 0.5rem;
  }
}

@media (min-width: 1264px) {
  .list-item {
    max-width: 100%;
    flex-direction: row;
    padding: 10px 10px;
    & .game-name {
      font-size: 1.5rem;
      margin-bottom: 1rem;
      width: 100%;
    }
    &__inner-text {
      display: block;
      padding: 0;
    }
  }
}
</style>
