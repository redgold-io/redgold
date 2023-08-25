<template>
  <div class="table-container">
    <table class="table table-striped table-hover">
      <thead>
      <tr>
        <th v-for="field in this.transactionFields" :key="field.key">
          {{ field.key.charAt(0).toUpperCase() + field.key.slice(1) }}
        </th>
      </tr>
      </thead>
      <tbody>
      <tr v-for="transaction in transactions" :key="transaction.hash">
        <td>
          <HashLink :data="transaction.hash" />
        </td>
        <td>
          <HashLink :data="transaction.from" />
        </td>
        <td>
          <HashLink :data="transaction.to" />
        </td>
        <td>
          {{ transaction.first_amount }}
        </td>
        <td>
          {{ transaction.fee }}
        </td>
        <td>
          {{ transaction.bytes }}
        </td>
        <td>{{ transaction.amount }}</td>
        <td>
          <RenderTime :timestamp="transaction.timestamp" />
        </td>
      </tr>
      </tbody>
    </table>
  </div>
</template>
<script>
import HashLink from "@/components/util/HashLink.vue";
import RenderTime from "@/components/RenderTime.vue";
export default {
  name: "BriefTransaction",
  components: {
    HashLink,
    RenderTime
  },
  props: {
    transactions: Array,
  },
  data() {
    return {
      transactionFields: [
        {key: 'hash'},
        {key: 'from'},
        {key: 'to'},
        {key: 'amount'},
        {key: 'fee'},
        {key: 'bytes'},
        {key: 'total'},
        {
          key: 'timestamp', formatter: (value) => {
            return new Date(value).toLocaleString(); // This line is assuming the timestamp is in milliseconds
          }
        },
      ],
    };
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