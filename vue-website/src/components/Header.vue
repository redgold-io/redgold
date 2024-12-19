<template>
  <div class="container-fluid">
    <nav class="navbar custom-navbar">
      <div class="navbar-container top-left-logo">
        <a class="navbar-brand brand  d-flex align-items-center" href="/">
          <img :src="require('@/assets/logo.png')" alt="Logo" class="logo">
          <div class="redgold-title">
            Red
          </div>
          <div class="redgold-title2">
            gold
          </div>
        </a>
        <a class="navbar-brand brand" href="https://dev.explorer.redgold.io">
          Explorer
        </a>
        <a class="navbar-brand brand" href="https://dev.docs.redgold.io">
          Docs
        </a>
        <a class="navbar-brand brand" href="https://discord.gg/86fzxJg8ce">
          Discord
        </a>

        <div class="navbar-right-links">
          <a class="navbar-brand brand" href="https://github.com/redgold-io/redgold">
            GitHub
          </a>
        </div>

      </div>

      <!-- Navbar items -->
<!--      <ul class="navbar-nav ml-auto">-->
<!--        <li class="nav-item">-->
<!--          <h6 class="nav-link">Home</h6>-->
<!--        </li>-->
<!--      </ul>-->
    </nav>
    <div class="row no-gutters">
      <!-- Buffer div -->
      <div class="col-1"></div>

      <!-- Main content div -->
      <div class="col-10">
      </div>
    </div>

  </div>
</template>

<script>

export default {
  name: 'HeaderBox',
  components: {
  },

}
</script>

<style scoped>

.top-left-logo {
  display: flex;
  align-items: center;
  gap: 20px;
  margin-left: 200px;
  //display: grid;
  //grid-template-columns: 2fr 1fr; /* Adjust as needed */
  //gap: 10px; /* Adjust as needed */
}

.redgold-title2 {
  font-size: 1.0em;
  font-weight: bold;
  //color: #ffcc00;
  //margin-top: 10px;
}

.redgold-title {
  font-size: 1.0em;
  font-weight: bold;
  //color: #bf3737;
  //margin-top: 10px;
}

.container-fluid {
  //background-color: #ededf0;
  //background-color: #2e2405;
  //background-color: #f1fa8c ;
  background-color: #29220c;
  //background-color: #44475a;
}

.logo {
  height: 55px;
  width: 55px;
  margin-right: 10px;
  //margin-left: 10px;
}

.custom-navbar .brand {
  color: #fff;
  user-select: none;
  text-decoration: none;
  padding-left: 10px;
  padding-right: 10px;
}

.custom-navbar .brand:hover {
  //background-color: #000;
  color: #fff;
}


/* Ensure the navbar container is using flexbox */
.navbar-container {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%; /* Make sure the container spans the full width */
  padding: 0; /* Remove padding to ensure alignment to the edges */
}

/* Align the right links to the far right */
.navbar-right-links {
  margin-left: auto; /* This will push the .navbar-right-links to the right */
  display: flex;
  align-items: center;
  gap: 20px;
  padding-right: 160px;
}

/* Existing styles */
.navbar-container {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%;
  padding: 0;
}
/* Responsive styles */
@media (max-width: 768px) {
  /* Adjust the breakpoint if needed */
  .navbar-container {
    flex-direction: row; /* Keep the row direction */
    flex-wrap: wrap; /* Allow the items to wrap to the next line */
    justify-content: center; /* Center items horizontally */
    align-items: center; /* Center items vertically */
  }

  .top-left-logo, .navbar-right-links {
    width: 100%; /* Make these elements take the full width */
    justify-content: center; /* Center the content */
    flex-wrap: wrap; /* Allow these items to wrap as well */
    margin-bottom: 10px; /* Add some space between the rows */
  }

  .custom-navbar .brand {
    flex: 1 1 auto; /* Allow the item to grow and shrink */
    max-width: 50%; /* Maximum width for each item */
    padding: 5px 10px; /* Adjust padding to reduce space */
    text-align: center; /* Center the text inside the buttons */
    box-sizing: border-box; /* Include padding and border in the element's total width and height */
  }

  .logo {
    height: 40px; /* Smaller logo for mobile */
    width: 40px;
    margin: 0 auto 10px; /* Center the logo and add margin at the bottom */
    display: block; /* Change display to block for the image */
  }

  .navbar-right-links {
    padding-right: 0; /* Remove padding */
  }
}



</style>
