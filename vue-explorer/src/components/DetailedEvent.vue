<template>
  <div class="table-container">
    <table class="table table-striped table-hover">
      <thead>
      <tr>
        <th v-for="field in this.transactionFields" :key="field">
          {{ field }}
        </th>
      </tr>
      </thead>
      <tbody>
      <tr v-for="transaction in reverseEvents" :key="transaction.tx_hash">
        <td>
          {{ transaction.event_type }}
        </td>
        <td>
          {{ transaction.extended_type }}
        </td>
        <td>
          {{ transaction.incoming }}
        </td>
        <td>
          {{ transaction.network }}
        </td>
        <td>
          <HashLink :data="transaction.other_address" :use-external-link="transaction.event_type === 'External'"/>
        </td>
        <td>
          <HashLink :data="transaction.tx_hash" :use-external-link="transaction.event_type === 'External'"
                    :is-address="false"
                    :bitcoin-external-link="transaction.other_address.startsWith('bc') || transaction.other_address.startsWith('tb')"
                    :ethereum-external-link="transaction.other_address.startsWith('0x')"
          />
        </td>
        <td>{{ transaction.amount }}</td>
        <td>
          <HashLink :data="transaction.other_tx_hash" :use-external-link="true"
                    :is-address="false"
                    :bitcoin-external-link="transaction.other_address.startsWith('bc') || transaction.other_address.startsWith('tb')"
                    :ethereum-external-link="transaction.other_address.startsWith('0x')"
          />
        </td>
        <td>
          {{ formatLocalTime(transaction.time) }}
        </td>
      </tr>
      </tbody>
    </table>
  </div>
</template>
<script>
import HashLink from "@/components/util/HashLink.vue";
// import RenderTime from "@/components/RenderTime.vue";
export default {
  name: "DetailedEvent",
  components: {
    HashLink,
    // TODO: Add time later
    // RenderTime
  },
  props: {
    events: Array,
  },
  methods: {
    formatLocalTime(milliseconds) {
      const date = new Date(milliseconds);
      return date.toLocaleString(undefined, {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
        hour12: true // Use this if you want 12-hour format
      });
    }
  },
  data() {
    return {
      transactionFields: [
          'Event Type',
          'Extended Type',
          'Incoming',
          'Network',
          'Other Address',
          'TX Hash',
          'Amount',
          'Other TX Hash',
          'Time'
      ],
    };
  },
  computed: {
    reverseEvents() {
      let events = this.events;
      return events.reverse();
    }
  }
}

</script>

<style>
.table .tr .td {
  color: #ffffff;
  background-color: #000000 !important;
}

.table-hover tbody tr:hover {
  color: #FFFFFF !important;
  background-color: #191a19 !important;
}

.table-striped>tbody>tr:nth-child(odd)>td,
.table-striped>tbody>tr:nth-child(odd)>th {
  background-color: #000000 !important;
  color: #ffffff;
}

.table-striped>tbody>tr:nth-child(even)>td,
.table-striped>tbody>tr:nth-child(even)>th {
  background-color: #191a19 !important;
  color: #ffffff;
}

.table-striped>tbody>tr:hover>td,
.table-striped>tbody>tr:hover>th {
  background-color: #291a00 !important; /* Set the desired color on hover */
  color: #ffffff;
}

th {
  background-color: #191a19 !important;
  color: #ffffff !important;
}

</style>