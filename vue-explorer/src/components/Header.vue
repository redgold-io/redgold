<template>
  <div class="container-fluid">
    <nav class="navbar custom-navbar">
      <a class="navbar-brand brand" href="/">
        <img :src="require('@/assets/logo.png')" alt="Logo" class="logo">
        Redgold Explorer
      </a>
      <!-- Navbar items -->
      <!-- <ul class="navbar-nav ml-auto">
        <li class="nav-item">
          <h6 class="nav-link">Home</h6>
        </li>
      </ul> -->
    </nav>
    <div class="row no-gutters">
      <!-- Buffer div -->
      <div class="col-1"></div>

      <!-- Main content div -->
      <div class="col-10">
        <!-- Search bar -->
        <div class="jumbotron search-bar py-4">
          <h5 class="text-light">Enter hash search query:</h5>
          <form class="d-flex align-items-center" :action="'/hash/' + searchValue" method="get">
<!--            <label for="inline-form-input-name" class="sr-only">Search</label>-->
            <input v-model="searchValue" id="inline-form-input-name" class="form-control mr-2 search-input" type="text" placeholder="Query hash...">
            <button type="submit" class="btn btn-primary">Submit</button>
          </form>
        </div>
      </div>
    </div>

  </div>
</template>

<script>
export default {
  name: 'HeaderBox',
  data() {
    return {
      searchValue: ''
    };
  },
  methods: {
    handleSubmit() {
      this.$router.push(`/hash/${this.searchValue}`);
    }
  }
}
</script>

<style scoped>
.custom-navbar {
  background-color: #000;
}

.logo {
  height: 50px;
  width: 50px;
  margin-right: 10px;
}

.custom-navbar .brand {
  color: #fff;
  user-select: none;
  text-decoration: none;
}

.custom-navbar .brand:hover {
  background-color: #000;
  color: #fff;
}

.search-bar {
  background-color: #000;
}

.search-input,
.search-input:focus {
  box-sizing: border-box;
  min-width: 600px;
  max-width: 600px;
  background-color: #191a19;
  color: #fff;
}
.search-input::placeholder {
  color: #ccc;
}

</style>
