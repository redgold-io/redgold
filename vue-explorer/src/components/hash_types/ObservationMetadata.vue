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
      <tr v-for="(om, index) in data" :key="index">
        <td><HashLink :data="om.observed_hash" /></td>
        <td> {{om.observed_hash_type}} </td>
        <td> {{om.validation_type}} </td>
        <td> {{om.state}} </td>
        <td> {{om.validation_confidence}} </td>
        <td> <RenderTime :timestamp="om.time"/></td>
        <td> {{shortenExeChecksum(om.metadata_hash)}} </td>
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
  name: "ObservationMetadata",
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
        {key: 'observed_hash'},
        {key: 'observed_hash_type'},
        {key: 'validation_type'},
        {key: 'state'},
        {key: 'validation_confidence'},
        {key: 'time'},
        {key: 'metadata_hash'}
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