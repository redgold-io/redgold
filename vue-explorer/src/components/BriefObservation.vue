<template>
  <div class="table-container">
    <table class="table table-striped table-hover">
      <thead>
      <tr>
        <th v-for="field in this.fields" :key="field.key">
          {{ formatTitle(field.key) }}
        </th>
      </tr>
      </thead>
      <tbody>
      <tr v-for="(d, index) in data" :key="index">
        <td><HashLink :data="d.public_key" /></td>
        <td><HashLink :data="d.hash" /></td>
        <td> {{d.observations.length}} </td>
        <td> {{d.height}} </td>
        <td><HashLink :data="d.parent_hash" /></td>
        <td><HashLink :data="d.observations[0].observed_hash" /></td>
        <td> <RenderTime :timestamp="d.time"/></td>
      </tr>
      </tbody>
    </table>
  </div>
</template>
<script>
import HashLink from "@/components/util/HashLink.vue";
import RenderTime from "@/components/RenderTime.vue";
import {toTitleCase} from "@/utils";
export default {
  name: "BriefObservation",
  components: {
    HashLink,
    RenderTime
  },
  props: {
    data: Object,
  },
  methods: {
    // Now you can use toTitleCase in this component
    formatTitle(key) {
      return toTitleCase(key);
    },
    shortenExeChecksum(h) {
      return h.substring(h.length - 8)
    }
  },
  data() {
    return {
      fields: [
        {key: 'public_key'},
        {key: 'observation_hash'},
        {key: 'count'},
        {key: 'height'},
        {key: 'parent_hash'},
        {key: 'sample'},
        {key: 'time'}
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